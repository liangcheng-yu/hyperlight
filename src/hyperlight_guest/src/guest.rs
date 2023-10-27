use crate::{
    flatbuffers::hyperlight::generated::{
        hlint, hlintArgs, root_as_function_call, root_as_guest_function_details, ErrorCode,
        FunctionCall, FunctionCallResult, FunctionCallResultArgs, FunctionCallType, GuestError,
        GuestErrorArgs, GuestFunctionDefinition, GuestFunctionDefinitionArgs, GuestFunctionDetails,
        GuestFunctionDetailsArgs, ParameterType, ParameterValue, ReturnType, ReturnValue,
    },
    hyperlight_peb::HyperlightPEB,
};

use core::{ffi::c_void, arch::asm};

use flatbuffers::{FlatBufferBuilder, ForwardsUOffset, UnionWIPOffset, WIPOffset};

extern crate alloc;
use alloc::vec::Vec;

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

struct HyperlightAllocator;

unsafe impl GlobalAlloc for HyperlightAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        hyperlight_more_core(size) as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: HyperlightAllocator = HyperlightAllocator;

unsafe fn hyperlight_more_core(size: usize) -> *mut c_void {
    static mut UNUSED_HEAP_BUFFER_POINTER: *mut u8 = null_mut();
    static mut ALLOCATED: usize = 0;

    if size > 0 {
        let ghs = (*P_PEB.unwrap()).guest_heap_data.guest_heap_size as usize;
        if ALLOCATED + size > ghs {
            set_error(ErrorCode::FailureInDlmalloc, "HyperlightMoreCore failed to allocate memory.");
        }

        let ptr: *mut u8 = if UNUSED_HEAP_BUFFER_POINTER.is_null() {
            (*P_PEB.unwrap()).guest_heap_data.guest_heap_buffer as *mut u8
        } else {
            UNUSED_HEAP_BUFFER_POINTER
        };

        ALLOCATED += size;
        UNUSED_HEAP_BUFFER_POINTER = ptr.add(size);
        return ptr as *mut c_void;
    }
    // Note: Here, we don't need to check size < 0, because, in Rust, that's disallowed
    // by the type-system.

    UNUSED_HEAP_BUFFER_POINTER as *mut c_void
}


static mut GUEST_FUNCTION_BUILDER: Option<FlatBufferBuilder> = None;

// This funciton either gets or initializes our guest function
// flatbuffer builder. In a sense, it replaces the need for
// to lazy_static, because of the issues we ran w/ compiling
// it in no_std/SUBSYSTEM:NATIVE.
fn get_or_init_guest_function_builder() -> &'static mut FlatBufferBuilder<'static> {
    unsafe {
        if GUEST_FUNCTION_BUILDER.is_none() {
            GUEST_FUNCTION_BUILDER = Some(FlatBufferBuilder::new());
        }

        GUEST_FUNCTION_BUILDER.as_mut().unwrap()
    }
}

fn reset_guest_function_builder() {
    unsafe {
        GUEST_FUNCTION_BUILDER = None;
    }
}

// These are global variable equivalents.
// Being in a single-threaded scenario, this should be fine.
static mut P_PEB: Option<*mut HyperlightPEB> = None;
static mut OS_PAGE_SIZE: u32 = 0;
static mut OUTB_PTR: Option<fn(u16, u8)> = None;
static mut OUTB_PTR_WITH_CONTEXT: Option<fn(*mut core::ffi::c_void, u16, u8)> = None;
static mut RUNNING_IN_HYPERLIGHT: bool = true;
static mut GUEST_FUNCTIONS: Option<GuestFunctionDetails> = None;

fn write_error(error_code: u64, message: Option<&str>) {
    let mut builder = flatbuffers::FlatBufferBuilder::new();

    let code = ErrorCode(error_code);

    let message_offset = message.map(|m| builder.create_string(m));
    
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
    let parameters = function_call.parameters();
    let parameter_count = parameters.unwrap().len();
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
                    parameter_kinds[i] = Some(ParameterType::hlint);
                }
                ParameterValue::hllong => {
                    parameter_kinds[i] = Some(ParameterType::hllong);
                }
                ParameterValue::hlstring => {
                    parameter_kinds[i] = Some(ParameterType::hlstring);
                }
                ParameterValue::hlbool => {
                    parameter_kinds[i] = Some(ParameterType::hlbool);
                }
                ParameterValue::hlvecbytes => {
                    parameter_kinds[i] = Some(ParameterType::hlvecbytes);
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
    let mut builder = get_or_init_guest_function_builder();

    let name = builder.create_string(function_name);

    let parameters = builder.create_vector(parameter_kinds);

    let return_type = ReturnType::hlint;

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
    let builder = get_or_init_guest_function_builder();
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

// In C's Flatbuffer world, this function
// is provided by the flatbuffer library itself.
// I don't think Rust has an equivalent, so I've
// created this for convenience, which I believe
// does the same thing (i.e., read the size prefix
// which is a u64, then read the data after the prefix
// in a provided flatbuffer vec of bytes).
fn read_size_prefixed_flatbuffer(buffer: *const u8) -> (usize, &'static [u8]) {
    assert!(!buffer.is_null());

    let size_prefix = unsafe { *(buffer as *const u64) } as usize;

    let data_slice = unsafe { core::slice::from_raw_parts(buffer.add(8), size_prefix) };

    (size_prefix, data_slice)
}

fn dispatch_function() {
    reset_error();

    let (_, buffer) = unsafe {
        read_size_prefixed_flatbuffer((*P_PEB.unwrap()).input_data.input_data_buffer as *const u8)
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
            (*P_PEB.unwrap()).output_data.output_data_buffer as *mut u8,
            result_size + 4,
        );

        // Note: I don't think the explicit dealloc here is necessary, as Rust
        // is a memory safe language, and the buffer will be dropped when it
        // goes out of scope.
    }
}

pub fn initialise_function_table() {
    let mut builder = get_or_init_guest_function_builder();
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

fn get_guest_function_details() -> GuestFunctionDetails<'static> {
    let builder = get_or_init_guest_function_builder();
    let buffer = builder.finished_data();

    // If you can assure that buffer is valid for the duration of its use
    root_as_guest_function_details(buffer).unwrap()
}

pub fn finalise_function_table() {
    // unsorted guest functions
    let guest_functions = get_guest_function_details();

    // get vector of function definitions
    let functions = guest_functions.functions();

    // - start sorting
    let num_functions = functions.len();

    let mut indices = Vec::<usize>::new();
    for i in 0..num_functions {
        indices[i] = i;
    }

    // sorting using indices, because we can't
    // really mess around w/ the flatbuffer vector.
    for i in 1..num_functions {
        let mut j = i;
        while j > 0
            && functions
                .get(indices[j - 1])
                .function_name()
                .cmp(&functions.get(indices[j]).function_name())
                == core::cmp::Ordering::Greater
        {
            indices.swap(j, j - 1);
            j -= 1;
        }
    }
    // ^^^ this is obviously not the best way ever to
    // sort, but it works for now.

    // - finished sorting

    reset_guest_function_builder();
    // ^^^ resetting, so we can use the sorted functions.
    for i in 0..num_functions {
        let parameters_vector = functions.get(indices[i]).parameters();
        let parameters_slice: Vec<_> = (0..parameters_vector.len())
            .map(|i| parameters_vector.get(i))
            .collect();
        // getting a slice

        let gfd = create_function_definition(
            functions.get(indices[i]).function_name(),
            functions.get(indices[i]).function_pointer() as u64,
            parameters_slice.as_slice(),
        );

        register_function(gfd);
    }
    let guest_functions = get_guest_function_details();

    unsafe {
        GUEST_FUNCTIONS = Some(guest_functions);
    }
}

fn halt() {
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
        let some_vec = Vec::from([1, 2, 3]);
        let _a = some_vec[0];
        
        // check if peb_address is null
          if peb_address == 0 {
            return -1;
        }

        P_PEB = Some(peb_address as *mut HyperlightPEB);

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

        // TODO: Here, in C, we call `dlmalloc_set_footprint_limit`,
        // our allocator might need something equivalent.

        reset_error();
        initialise_function_table(); // <- unlike in C, this can't fail in Rust

        hyperlight_main();
        finalise_function_table();

        (*(P_PEB.unwrap())).output_data.output_data_buffer = 0 as *mut c_void;
    }

    halt();
    0
}
