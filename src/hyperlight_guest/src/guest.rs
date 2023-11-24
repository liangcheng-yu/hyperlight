use crate::hyperlight_peb::HyperlightPEB;

use core::{
    arch::asm,
    ffi::c_void,
    ptr::copy_nonoverlapping,
    slice::from_raw_parts,
    sync::atomic::{AtomicPtr, Ordering},
};

use flatbuffers::{size_prefixed_root, FlatBufferBuilder, UnionWIPOffset, WIPOffset};

use alloc::{string::ToString, vec::Vec};

use buddy_system_allocator::LockedHeap;
use hyperlight_flatbuffers::{
    flatbuffer_wrappers::guest_function_details::GuestFunctionDetails,
    flatbuffers::hyperlight::generated::size_prefixed_root_as_function_call_result,
};
use hyperlight_flatbuffers::{
    flatbuffer_wrappers::{
        function_types::{ParameterType, ReturnType},
        guest_function_definition::GuestFunctionDefinition,
    },
    flatbuffers::hyperlight::generated::{
        hlint as Fbhlint, hlintArgs as FbhlintArgs, ErrorCode as FbErrorCode,
        FunctionCall as FbFunctionCall, FunctionCallResult as FbFunctionCallResult,
        FunctionCallResultArgs as FbFunctionCallResultArgs, FunctionCallType as FbFunctionCallType,
        GuestError as FbGuestError, GuestErrorArgs as FbGuestErrorArgs,
        GuestFunctionDetails as FbGuestFunctionDetails, ParameterType as FbParameterType,
        ParameterValue as FbParameterValue, ReturnValue as FbReturnValue,
    },
};

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::<32>::empty();

static P_PEB: AtomicPtr<HyperlightPEB> = AtomicPtr::new(core::ptr::null_mut());

static mut OS_PAGE_SIZE: u32 = 0;
static mut OUTB_PTR: Option<fn(u16, u8)> = None;
static mut OUTB_PTR_WITH_CONTEXT: Option<fn(*mut core::ffi::c_void, u16, u8)> = None;
static mut RUNNING_IN_HYPERLIGHT: bool = true;
static mut GUEST_FUNCTIONS: Option<GuestFunctionDetails> = None;
static mut GUEST_FUNCTIONS_FINALISED: Option<Vec<u8>> = None;

fn write_error(error_code: u64, message: Option<&str>) {
    let peb_ptr = P_PEB.load(Ordering::SeqCst);
    let mut builder = flatbuffers::FlatBufferBuilder::new();

    let code = FbErrorCode(error_code);

    let message_offset: Option<WIPOffset<&str>> = message.map(|m| builder.create_string(m));

    let error = FbGuestError::create(
        &mut builder,
        &FbGuestErrorArgs {
            code,
            message: message_offset,
        },
    );

    builder.finish_size_prefixed(error, None);

    let flatb_data = builder.finished_data();

    unsafe {
        assert!(flatb_data.len() <= (*peb_ptr).guestErrorBufferSize as usize);
    }

    unsafe {
        assert!(!(*peb_ptr).pGuestErrorBuffer.is_null());
        assert!(flatb_data.len() <= (*peb_ptr).guestErrorBufferSize as usize);

        // Optimally, we'd use copy_from_slice here, but, because
        // p_guest_error_buffer is a *mut c_void, we can't do that.
        // Instead, we do the copying manually using pointer arithmetic.
        // Plus; before, we'd do an assert w/ the result from copy_from_slice,
        // but, because copy_nonoverlapping doesn't return anything, we can't do that.
        // Instead, we do the prior asserts to check the destination pointer isn't null
        // and that there is enough space in the destination buffer for the copy.
        let dest_ptr = (*peb_ptr).pGuestErrorBuffer as *mut u8;
        core::ptr::copy_nonoverlapping(flatb_data.as_ptr(), dest_ptr, flatb_data.len());
    }
}

fn reset_error() {
    write_error(0, None);
}

fn set_error(error_code: FbErrorCode, message: &str) {
    let peb_ptr = P_PEB.load(Ordering::SeqCst);
    write_error(error_code.0, Some(message));
    unsafe {
        (*peb_ptr).outputdata.outputDataBuffer = usize::MAX as *mut c_void;
    }
}

type GuestFunc = fn() -> Vec<u8>;
fn call_guest_function(function_call: &FbFunctionCall) -> Vec<u8> {
    let gfd = unsafe { GUEST_FUNCTIONS_FINALISED.as_ref().unwrap() };
    let guest_function_definitions = size_prefixed_root::<FbGuestFunctionDetails>(gfd)
        .unwrap()
        .functions();

    let parameters = function_call.parameters();
    let function_name = function_call.function_name();

    if let Some(key) = guest_function_definitions
        .iter()
        .position(|func_def| func_def.function_name() == function_name)
    {
        let function_definition = &guest_function_definitions.get(key);
        let p_function = unsafe {
            let function_pointer = function_definition.function_pointer();
            core::mem::transmute::<i64, GuestFunc>(function_pointer)
        };

        let parameter_types = function_definition.parameters();

        // As Hyperlight only supports up to 10 parameters, we can use a fixed size array
        const MAX_PARAMETERS: usize = 10;
        let required_parameter_count = parameter_types.len();
        assert!(
            required_parameter_count <= MAX_PARAMETERS,
            "Exceeded maximum parameter count"
        );

        if let Some(p) = parameters {
            let parameter_count = p.len();
            if required_parameter_count != parameter_count {
                panic!(
                    "Called function {} with {} parameters but it takes {}.",
                    function_name, parameter_count, required_parameter_count
                );
            }
        }

        let mut parameter_kinds: [Option<FbParameterType>; 10] = [None; MAX_PARAMETERS];
        let mut next_param_is_length = false;

        for i in 0..required_parameter_count {
            let parameter = &parameters.unwrap().get(i);
            let parameter_type = parameter.value_type();

            if next_param_is_length {
                if parameter_type != FbParameterValue::hlint {
                    panic!("Parameter {}", i);
                }
                next_param_is_length = false;
            }

            match parameter_type {
                FbParameterValue::hlint => {
                    parameter_kinds[i] = Some(FbParameterType::hlint);
                }
                FbParameterValue::hllong => {
                    parameter_kinds[i] = Some(FbParameterType::hllong);
                }
                FbParameterValue::hlstring => {
                    parameter_kinds[i] = Some(FbParameterType::hlstring);
                }
                FbParameterValue::hlbool => {
                    parameter_kinds[i] = Some(FbParameterType::hlbool);
                }
                FbParameterValue::hlvecbytes => {
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

        return p_function();
    } else {
        extern "C" {
            #[allow(improper_ctypes)]
            fn guest_dispatch_function() -> Vec<u8>;
        }

        // If the function was not found call the GuestDispatchFunction method.
        unsafe {
            return guest_dispatch_function();
        }
    }
}

pub fn get_flatbuffer_result_from_int(value: i32) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let hlint = Fbhlint::create(&mut builder, &FbhlintArgs { value });

    let rt = FbReturnValue::hlint;
    let rv: Option<WIPOffset<UnionWIPOffset>> = Some(hlint.as_union_value());

    get_flatbuffer_result(&mut builder, rt, rv)
}

fn get_flatbuffer_result(
    builder: &mut FlatBufferBuilder,
    return_value_type: FbReturnValue,
    return_value: Option<WIPOffset<UnionWIPOffset>>,
) -> Vec<u8> {
    let result_offset = FbFunctionCallResult::create(
        builder,
        &FbFunctionCallResultArgs {
            return_value,
            return_value_type,
        },
    );

    builder.finish_size_prefixed(result_offset, None);

    builder.finished_data().to_vec()
}

fn dispatch_function() {
    reset_error();
    let peb_ptr = P_PEB.load(Ordering::SeqCst);

    let idb = unsafe {
        from_raw_parts(
            (*peb_ptr).inputdata.inputDataBuffer as *mut u8,
            (*peb_ptr).inputdata.inputDataSize as usize,
        )
    };
    let function_call = flatbuffers::size_prefixed_root::<FbFunctionCall>(idb).unwrap();

    // Validate this is a Guest Function Call
    if function_call.function_call_type() != FbFunctionCallType::guest {
        set_error(FbErrorCode::GuestError, "Invalid Function Call Type");
        return;
    }

    let result_vec = call_guest_function(&function_call);
    size_prefixed_root_as_function_call_result(&result_vec).unwrap();

    unsafe {
        let output_data_buffer =
            (*P_PEB.load(Ordering::SeqCst)).outputdata.outputDataBuffer as *mut u8;
        let size_with_prefix = result_vec.len();

        copy_nonoverlapping(result_vec.as_ptr(), output_data_buffer, size_with_prefix);
    }
}

pub fn initialise_function_table() {
    unsafe {
        GUEST_FUNCTIONS = Some(GuestFunctionDetails::new(Vec::new()));
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
        gfs.insert(function_definition);
    } else {
        // it's impossible for this to happen because we always initialise the function table
        // prior to calling the guest's hyperlight_main, but just in case.
        initialise_function_table();
    }
}

pub fn finalise_function_table() {
    unsafe {
        GUEST_FUNCTIONS_FINALISED = Some((GUEST_FUNCTIONS.as_ref().unwrap()).try_into().unwrap());
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
        // check if peb_address is null
        if peb_address == 0 {
            return -1;
        }

        P_PEB.store(peb_address as *mut HyperlightPEB, Ordering::SeqCst);
        let peb_ptr = P_PEB.load(Ordering::SeqCst);

        let heap_start = (*peb_ptr).guestheapData.guestHeapBuffer as usize;
        let heap_size = (*peb_ptr).guestheapData.guestHeapSize as usize;
        HEAP_ALLOCATOR.lock().init(heap_start, heap_size);

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
        OUTB_PTR = Some(core::mem::transmute((*peb_ptr).pOutb));
        OUTB_PTR_WITH_CONTEXT = Some(core::mem::transmute((*peb_ptr).pOutb));

        // If outb is not null, then we're not running in Hyperlight
        if !(*peb_ptr).pOutb.is_null() {
            RUNNING_IN_HYPERLIGHT = false;
        }

        (*peb_ptr).guest_function_dispatch_ptr = dispatch_function as u64;

        reset_error();
        initialise_function_table();

        hyperlight_main();
        finalise_function_table();

        // (*peb_ptr).outputdata.outputDataBuffer = 0 as *mut c_void;
    }

    0
}
