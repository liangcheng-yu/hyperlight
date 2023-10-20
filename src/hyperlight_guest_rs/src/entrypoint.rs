use crate::{
    flatbuffers::hyperlight::generated::{
        hlint, hlintArgs, root_as_function_call, root_as_guest_function_details, ErrorCode,
        FunctionCall, FunctionCallResult, FunctionCallResultArgs, FunctionCallType, GuestError,
        GuestErrorArgs, GuestFunctionDefinition, GuestFunctionDefinitionArgs, GuestFunctionDetails,
        GuestFunctionDetailsArgs, ParameterType, ParameterValue, ReturnType, ReturnValue,
    },
    hyperlight_peb::HyperlightPEB,
};
use core::alloc::GlobalAlloc;
use core::{alloc::Layout, ffi::c_void};
use flatbuffers::{FlatBufferBuilder, ForwardsUOffset, UnionWIPOffset, WIPOffset};
use lazy_static::lazy_static;
use mimalloc::MiMalloc;
use spin::RwLock;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

lazy_static! {
    static ref GUEST_FUNCTION_BUILDER: RwLock<FlatBufferBuilder<'static>> =
        RwLock::new(FlatBufferBuilder::new());
}

// `static mut`s like this only work in single-threaded scenarios.
// In multi-threaded scenarios, we use the standard library primitives, but
// because we're in a `#![no_std]` environment, we can't use those.

static mut P_PEB: Option<*mut HyperlightPEB> = None;
static mut RUNNING_IN_HYPERLIGHT: bool = true;
static mut OUTB_PTR: Option<fn(u16, u8)> = None;
static mut OUTB_PTR_WITH_CONTEXT: Option<fn(*mut core::ffi::c_void, u16, u8)> = None;
static mut GUEST_FUNCTIONS: Option<GuestFunctionDetails> = None;
// ^^^ These might be in lazy_static too inside of a RwLock to make them thread-safe.

fn write_error(error_code: u64, message: Option<&str>) {
    // Create a flatbuffer builder object
    let mut builder = flatbuffers::FlatBufferBuilder::new();

    // Validate the error code
    let code = ErrorCode(error_code as _);

    // Create the flatbuffer
    let message_offset = message.map(|m| builder.create_string(m));
    let error = GuestError::create(
        &mut builder,
        &GuestErrorArgs {
            code,
            message: message_offset,
            ..Default::default() // fill in other fields as necessary
        },
    );

    builder.finish(error, None);

    let flatb_data = builder.finished_data();
    unsafe {
        assert!(flatb_data.len() <= (*P_PEB.unwrap()).guest_error_buffer_size as usize);
    }

    unsafe {
        assert!(!(*P_PEB.unwrap()).p_guest_error_buffer.is_null());
        assert!(flatb_data.len() <= (*P_PEB.unwrap()).guest_error_buffer_size as usize);

        // Optimally, we'd use copy_from_slice here, but because
        // p_guest_error_buffer is a *mut c_void, we can't do that.
        // Instead, we do the copying manually using pointer arithmetic.
        // Plus, before we'd do an assert w/ the result from copy_from_slice,
        // but copy_nonoverlapping doesn't return anything, so we can't do that.
        // Instead, we do the prior asserts to check the destination pointer isn't null
        // and that there is enough space in the destination buffer for the copy.
        let dest_ptr = (*P_PEB.unwrap()).p_guest_error_buffer as *mut u8;
        core::ptr::copy_nonoverlapping(flatb_data.as_ptr(), dest_ptr, flatb_data.len());
    }
}

fn reset_error() {
    write_error(0, None);
}

fn set_error(error_code: ErrorCode, message: &str) {
    write_error(error_code.0, Some(message));
    unsafe {
        (*P_PEB.unwrap()).outputdata.output_data_buffer = usize::MAX as *mut c_void;
    }
}

type GuestFunc = fn(FunctionCall) -> *mut u8;
fn call_guest_function(function_call: &FunctionCall) -> *mut u8 {
    let parameters = function_call.parameters();
    let actual_parameter_count = parameters.unwrap().len();
    let function_name = function_call.function_name();

    let guest_function_definitions = unsafe { GUEST_FUNCTIONS.unwrap().functions() };

    if let Some(key) = guest_function_definitions
        .iter()
        .position(|func_def| func_def.function_name() == function_name)
    {
        let function_definition = &guest_function_definitions.get(key);
        let p_function = unsafe {
            core::mem::transmute::<i64, GuestFunc>(function_definition.function_pointer())
        };

        let parameter_types = function_definition.parameters();
        // As Hyperlight only supports up to 10 parameters, we can use a fixed size array,
        // which is great because we can't use Vec due to no_std.
        const MAX_PARAMETERS: usize = 10;
        let required_parameter_count = parameter_types.len();
        assert!(
            MAX_PARAMETERS <= required_parameter_count,
            "Exceeded maximum parameter count"
        );

        if required_parameter_count != actual_parameter_count {
            panic!(
                "Called function {} with {} parameters but it takes {}.",
                function_name, actual_parameter_count, required_parameter_count
            );
        }

        let mut parameter_kinds = [None; MAX_PARAMETERS];
        let mut index = 0;
        let mut next_param_is_length = false;

        for i in 0..required_parameter_count {
            let parameter = &parameters.unwrap().get(i);
            let parameter_type = parameter.value_type();

            if next_param_is_length {
                if parameter_type != ParameterValue::hlint {
                    panic!("Parameter {}", i);
                }
                next_param_is_length = false;
            }

            match parameter_type {
                ParameterValue::hlint => {
                    parameter_kinds[index] = Some(ParameterType::hlint);
                    index += 1;
                }
                ParameterValue::hllong => {
                    parameter_kinds[index] = Some(ParameterType::hllong);
                    index += 1;
                }
                ParameterValue::hlstring => {
                    parameter_kinds[index] = Some(ParameterType::hlstring);
                    index += 1;
                }
                ParameterValue::hlbool => {
                    parameter_kinds[index] = Some(ParameterType::hlbool);
                    index += 1;
                }
                ParameterValue::hlvecbytes => {
                    parameter_kinds[index] = Some(ParameterType::hlvecbytes);
                    next_param_is_length = true;
                    index += 1;
                }
                _ => panic!(
                    "Unexpected Parameter Type {:#?} in Function {}",
                    parameter_type, function_name
                ),
            }
        }

        if next_param_is_length {
            panic!("Last parameter should be the length of the array");
        }

        for i in 0..required_parameter_count {
            if *parameter_kinds[i].as_ref().unwrap() != parameter_types.get(i) {
                panic!("Function {} parameter {}.", function_name, i);
            }
        }

        return p_function(*function_call);
    } else {
        extern "C" {
            fn guest_dispatch_function(function_call: &FunctionCall) -> *mut u8;
        }

        // If the function was not found call the GuestDispatchFunction method.
        unsafe {
            return guest_dispatch_function(function_call);
        }
    }
}

pub fn create_function_definition(
    function_name: &str,
    p_function: u64,
    parameter_kinds: &[ParameterType],
) -> WIPOffset<GuestFunctionDefinition<'static>> {
    let mut builder = GUEST_FUNCTION_BUILDER.write();

    // Create the name string
    let name = builder.create_string(function_name);

    // Create the parameter types vector
    let parameters = builder.create_vector(parameter_kinds);

    let return_type = ReturnType::hlint;

    // Create the GuestFunctionDefinition and return its offset
    GuestFunctionDefinition::create(
        &mut builder,
        &GuestFunctionDefinitionArgs {
            function_name: Some(name),
            parameters: Some(parameters),
            return_type: return_type,
            function_pointer: p_function as i64,
            ..Default::default()
        },
    )
}

pub fn register_function(function_definition: WIPOffset<GuestFunctionDefinition<'static>>) {
    let mut builder = GUEST_FUNCTION_BUILDER.write();
    builder.push(function_definition); 
}


pub fn get_flatbuffer_result_from_int(value: u32) -> *mut u8 {
    let mut builder = FlatBufferBuilder::new();
    let hlint = hlint::create(
        &mut builder,
        &hlintArgs {
            value: value as i32,
        },
    );

    let rt = ReturnValue::hlint;
    let rv = Some(hlint.as_union_value());

    get_flatbuffer_result(&mut builder, rt, rv)
}

fn get_flatbuffer_result(
    builder: &mut FlatBufferBuilder,
    return_value_type: ReturnValue,
    return_value: Option<WIPOffset<UnionWIPOffset>>,
) -> *mut u8 {
    let result_offset = FunctionCallResult::create(
        builder,
        &FunctionCallResultArgs {
            return_value,
            return_value_type,
            ..Default::default()
        },
    );

    builder.finish(result_offset, None);

    builder.finished_data().as_ptr() as *mut u8
}

fn read_size_prefixed_flatbuffer(buffer: *const u8) -> (usize, &'static [u8]) {
    assert!(!buffer.is_null());

    // Read the size prefix.
    let size_prefix = unsafe { *(buffer as *const u64) } as usize;

    // Convert the raw pointer (after the prefix) to a slice.
    let data_slice = unsafe { core::slice::from_raw_parts(buffer.add(8), size_prefix) };

    (size_prefix, data_slice)
}

fn dispatch_function() {
    reset_error();

    // Read the Function Call FlatBuffer from memory
    let (_, buffer) = unsafe {
        read_size_prefixed_flatbuffer((*P_PEB.unwrap()).inputdata.input_data_buffer as *const u8)
    };

    let function_call = root_as_function_call(buffer).unwrap();

    // Validate this is a Guest Function Call
    if function_call.function_call_type() != FunctionCallType::guest {
        set_error(ErrorCode::GuestError, "Invalid Function Call Type");
        return;
    }

    let result = call_guest_function(&function_call);
    let (result_size, result_buffer) = read_size_prefixed_flatbuffer(result);

    unsafe {
        core::ptr::copy(
            result_buffer.as_ptr(),
            (*P_PEB.unwrap()).outputdata.output_data_buffer as *mut u8,
            result_size + 4,
        );

        GLOBAL.dealloc(
            result as *mut u8,
            Layout::from_size_align_unchecked(result_size + 4, 1),
        );
    }
}

pub fn initialise_function_table() {
    let mut builder = GUEST_FUNCTION_BUILDER.write();
    let functions_vec = builder.create_vector::<ForwardsUOffset<GuestFunctionDefinition>>(&[]);

    let details = GuestFunctionDetails::create(
        &mut builder,
        &GuestFunctionDetailsArgs {
            functions: Some(functions_vec),
            ..Default::default()
        },
    );

    builder.finish(details, None);
}

// Assuming a maximum size for your flatbuffers data.
const MAX_FLATBUFFER_SIZE: usize = 4096;
static mut FLATBUFFER_STORAGE: [u8; MAX_FLATBUFFER_SIZE] = [0; MAX_FLATBUFFER_SIZE];

pub fn finalise_function_table() {
    {
        let builder = GUEST_FUNCTION_BUILDER.write();
        let buffer = builder.finished_data();

        // Copy the buffer data to the static storage
        unsafe {
            FLATBUFFER_STORAGE[..buffer.len()].copy_from_slice(buffer);
        }
    }

    let guest_functions = unsafe { root_as_guest_function_details(&FLATBUFFER_STORAGE) }.unwrap();

    let _guest_function_definitions = guest_functions.functions();

    // TODO: Sort functions by name.

    // Store the sorted buffer for later use (this is pseudo-code, adjust as needed).
    unsafe {
        GUEST_FUNCTIONS = Some(guest_functions);
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn entryPoint(peb_address: i64, _seed: i64, _ops: i32) -> i32 {
    unsafe {
        P_PEB = Some(peb_address as *mut HyperlightPEB);

        if P_PEB.is_none() {
            return -1;
        }

        // In C, at this point, we call __security_init_cookie.
        // That's a dependency on MSVC, which we can't utilize here.
        // This is to protect against buffer overflows in C, which
        // are inherently protected in Rust.

        // In C, here, we have a `if (!setjmp(jmpbuf))`, which is used in case an error occurs
        // because longjmp is called, which will cause execution to return to this point to
        // halt the program. In Rust, we don't have this sort of error handling as the
        // language relies on specific structures like `Result`, and `?` that allow for
        // propagating up the call stack.

        OUTB_PTR = Some(core::mem::transmute::<_, fn(u16, u8)>(
            (*P_PEB.unwrap()).p_outb,
        ));
        OUTB_PTR_WITH_CONTEXT = if (*P_PEB.unwrap()).p_outb_context.is_null() {
            None
        } else {
            Some(core::mem::transmute((*P_PEB.unwrap()).p_outb))
        };

        if let Some(_) = OUTB_PTR_WITH_CONTEXT {
            RUNNING_IN_HYPERLIGHT = false;
        }

        (*P_PEB.unwrap()).guest_function_dispatch_ptr = dispatch_function as u64;

        // Note: Here, in C, we call `dlmalloc_set_footprint_limit`, but I
        // don't think mimalloc has an equivalent.

        reset_error();
        initialise_function_table(); // <- unlike in C, this can't fail in Rust

        extern "C" {
            fn hyperlight_main();
        }

        hyperlight_main();
        finalise_function_table();

        (*(P_PEB.unwrap())).outputdata.output_data_buffer = 0 as *mut c_void;
    }

    0
}
