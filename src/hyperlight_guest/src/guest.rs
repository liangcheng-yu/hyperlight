use crate::{
    function::{GuestFunctionDefinition, GuestFunctionDetails, ParameterType, ReturnType},
    gen_flatbuffers::hyperlight::generated::{
        hlint, hlintArgs, root_as_function_call, ErrorCode, FunctionCall, FunctionCallResult,
        FunctionCallResultArgs, FunctionCallType, GuestError, GuestErrorArgs,
        GuestFunctionDetails as FbGuestFunctionDetails, ParameterType as FbParameterType,
        ParameterValue, ReturnValue,
    },
    hyperlight_peb::HyperlightPEB,
};

use core::{arch::asm, ffi::c_void};

use flatbuffers::{root, FlatBufferBuilder, UnionWIPOffset, WIPOffset};

use alloc::{string::ToString, vec::Vec};

use buddy_system_allocator::LockedHeap;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::<32>::empty();

// These are global variable equivalents.
// Being in a single-threaded scenario, this should be fine.
static mut P_PEB: Option<*mut HyperlightPEB> = None;
static mut OS_PAGE_SIZE: u32 = 0;
static mut OUTB_PTR: Option<fn(u16, u8)> = None;
static mut OUTB_PTR_WITH_CONTEXT: Option<fn(*mut core::ffi::c_void, u16, u8)> = None;
static mut RUNNING_IN_HYPERLIGHT: bool = true;
static mut GUEST_FUNCTIONS: Option<GuestFunctionDetails> = None;
static mut GUEST_FUNCTIONS_FINALISED: Option<Vec<u8>> = None;

fn write_error(error_code: u64, message: Option<&str>) {
    let mut builder = flatbuffers::FlatBufferBuilder::new();

    let code = ErrorCode(error_code);

    let message_offset: Option<WIPOffset<&str>> = message.map(|m| builder.create_string(m));

    let error = GuestError::create(
        &mut builder,
        &GuestErrorArgs {
            code,
            message: message_offset,
        },
    );

    builder.finish_size_prefixed(error, None);

    let flatb_data = builder.finished_data();

    unsafe {
        assert!(flatb_data.len() <= (*P_PEB.unwrap()).guest_error_buffer_size as usize);
    }

    unsafe {
        assert!(!(*P_PEB.unwrap()).p_guest_error_buffer.is_null());
        assert!(flatb_data.len() <= (*P_PEB.unwrap()).guest_error_buffer_size as usize);

        // Optimally, we'd use copy_from_slice here, but, because
        // p_guest_error_buffer is a *mut c_void, we can't do that.
        // Instead, we do the copying manually using pointer arithmetic.
        // Plus; before, we'd do an assert w/ the result from copy_from_slice,
        // but, because copy_nonoverlapping doesn't return anything, we can't do that.
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
        (*P_PEB.unwrap()).output_data.output_data_buffer = usize::MAX as *mut c_void;
    }
}

type GuestFunc = fn(FunctionCall) -> *mut u8;
fn call_guest_function(function_call: &FunctionCall) -> *mut u8 {
    let gfd = unsafe { GUEST_FUNCTIONS_FINALISED.as_ref().unwrap() };
    let guest_function_definitions = root::<FbGuestFunctionDetails>(gfd).unwrap().functions();

    let parameters = function_call.parameters();
    let parameter_count = parameters.unwrap().len();
    let function_name = function_call.function_name();

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

        if required_parameter_count != parameter_count {
            panic!(
                "Called function {} with {} parameters but it takes {}.",
                function_name, parameter_count, required_parameter_count
            );
        }

        let mut parameter_kinds = [None; MAX_PARAMETERS];
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
                    parameter_kinds[i] = Some(FbParameterType::hlint);
                }
                ParameterValue::hllong => {
                    parameter_kinds[i] = Some(FbParameterType::hllong);
                }
                ParameterValue::hlstring => {
                    parameter_kinds[i] = Some(FbParameterType::hlstring);
                }
                ParameterValue::hlbool => {
                    parameter_kinds[i] = Some(FbParameterType::hlbool);
                }
                ParameterValue::hlvecbytes => {
                    parameter_kinds[i] = Some(FbParameterType::hlvecbytes);
                    next_param_is_length = true;
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
            #[allow(improper_ctypes)]
            fn guest_dispatch_function(function_call: &FunctionCall) -> &'static [u8];
        }

        // If the function was not found call the GuestDispatchFunction method.
        unsafe {
            return guest_dispatch_function(function_call);
        }
    }
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
    let rv: Option<WIPOffset<UnionWIPOffset>> = Some(hlint.as_union_value());

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

    builder.finish_size_prefixed(result_offset, None);

    builder.finished_data().as_ptr() as *mut u8
}

fn dispatch_function() {
    reset_error();

    let idb = unsafe { &(*P_PEB.unwrap()).input_data.input_data_buffer };
    let function_call = flatbuffers::size_prefixed_root::<FunctionCall>(&idb).unwrap();

    // Validate this is a Guest Function Call
    if function_call.function_call_type() != FunctionCallType::guest {
        set_error(ErrorCode::GuestError, "Invalid Function Call Type");
        return;
    }

    let result = call_guest_function(&function_call);
    let result = flatbuffers::size_prefixed_root::<FunctionCallResult>(result).unwrap();

    unsafe {
        core::ptr::copy(
            result_buffer.as_ptr(),
            (*P_PEB.unwrap()).output_data.output_data_buffer as *mut u8,
            result_size + 4,
        );

        // Note: I don't think the explicit dealloc here is necessary, as Rust
        // is a memory safe language, and the buffer will be dropped when it
        // goes out of scope.
    }
}

pub fn initialise_function_table() {
    unsafe {
        GUEST_FUNCTIONS = Some(GuestFunctionDetails::new());
    }
}

pub fn create_function_definition(
    function_name: &str,
    p_function: i64,
    parameters: &[ParameterType],
) -> GuestFunctionDefinition {
    GuestFunctionDefinition::new(
        function_name.to_string(),
        parameters.to_vec(),
        ReturnType::Int, // HL's Guest Lib only supports Int return types for now
        p_function,
    )
}

pub fn register_function(function_definition: GuestFunctionDefinition) {
    if let Some(gfs) = unsafe { GUEST_FUNCTIONS.as_mut() } {
        gfs.insert_guest_function(function_definition);
    } else {
        // it's impossible for this to happen because we always initialise the function table
        // prior to calling the guest's hyperlight_main, but just in case.
        initialise_function_table();
    }
}

pub fn finalise_function_table() {
    unsafe {
        GUEST_FUNCTIONS_FINALISED = Some((GUEST_FUNCTIONS.as_mut().unwrap()).into());
    }
}

pub fn halt() {
    unsafe {
        asm!("hlt");
    }
}

extern "C" {
    fn hyperlight_main();
}

#[no_mangle]
pub extern "C" fn entrypoint(peb_address: u64, _seed: u64, ops: i32) -> i32 {
    unsafe {
        P_PEB = Some(peb_address as *mut HyperlightPEB);

        let heap_start = (*P_PEB.unwrap()).guest_heap_data.guest_heap_buffer as usize;
        let heap_size = (*P_PEB.unwrap()).guest_heap_data.guest_heap_size as usize;
        HEAP_ALLOCATOR.lock().init(heap_start, heap_size);

        let mut some_vec = Vec::from([1, 2, 3]);

        let some_other_vec = Vec::from([4, 5, 6]);

        // copy_from_slice some_other_vec into some_vec
        some_vec.copy_from_slice(&some_other_vec);

        // check if peb_address is null
        if peb_address == 0 {
            return -1;
        }

        // In C, at this point, we call __security_init_cookie.
        // That's a dependency on MSVC, which we can't utilize here.
        // This is to protect against buffer overflows in C, which
        // are inherently protected in Rust.

        // In C, here, we have a `if (!setjmp(jmpbuf))`, which is used in case an error occurs
        // because longjmp is called, which will cause execution to return to this point to
        // halt the program. In Rust, we don't have or need this sort of error handling as the
        // language relies on specific structures like `Result`, and `?` that allow for
        // propagating up the call stack.

        OS_PAGE_SIZE = ops as u32;
        OUTB_PTR = Some(core::mem::transmute((*P_PEB.unwrap()).p_outb));
        OUTB_PTR_WITH_CONTEXT = Some(core::mem::transmute((*P_PEB.unwrap()).p_outb));

        // If outb is not null, then we're not running in Hyperlight
        if !(*P_PEB.unwrap()).p_outb.is_null() {
            RUNNING_IN_HYPERLIGHT = false;
        }

        (*P_PEB.unwrap()).guest_function_dispatch_ptr = dispatch_function as u64;

        reset_error();
        initialise_function_table();

        hyperlight_main();
        finalise_function_table();

        (*(P_PEB.unwrap())).output_data.output_data_buffer = 0 as *mut c_void;
    }

    0
}
