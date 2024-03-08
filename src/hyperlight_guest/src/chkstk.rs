use crate::{guest_error::set_stack_allocate_error, MIN_STACK_ADDRESS, RUNNING_IN_HYPERLIGHT};
use core::arch::global_asm;

extern "win64" {
    fn __chkstk();
    fn __chkstk_in_proc();
}

global_asm!(
    "
    .global __chkstk
    __chkstk:
        /* Make space on the stack and save R10 and R11 */
        sub rsp, 0x10
        mov qword ptr [rsp], r10
        mov qword ptr [rsp + 8], r11
        /* Check if we are running in Hyperlight */
        lea r11,[rip+{running_in_hyperlight}]
        movzx r11,byte ptr [r11]
        test r11,r11
        /* If we are not running in Hyperlight, jump to call_chk_inproc */
        je call_chk_inproc
        /* Load the minimum stack address from the PEB */
        lea r11,[rip+{min_stack_addr}]  
        mov r11, qword ptr [r11]
        /* Get the current stack pointer */
        lea r10,[rsp+0x18]  
        /* Calculate what the new stack pointer will be */
        sub r10, rax
        /* If result is negative, cause StackOverflow */
        js call_set_error
        /* Compare the new stack pointer with the minimum stack address */
        cmp r10,r11   
        /* If the new stack pointer is above the minimum stack address, jump to cs_ret */
        jae cs_ret
        /* If the new stack pointer is below the minimum stack address, 
        then set the error code to 9 (stack overflow) call set_error and halt */
    call_set_error:
        call {set_error}
        hlt
    call_chk_inproc:
        call {chkstk_in_proc}
    cs_ret:
        /* Restore R10 and R11 and return */
        mov r10, qword ptr [rsp]
        mov r11, qword ptr [rsp + 8]
        add rsp, 0x10
        ret
",
        running_in_hyperlight = sym RUNNING_IN_HYPERLIGHT,
        chkstk_in_proc = sym __chkstk_in_proc,
        min_stack_addr = sym MIN_STACK_ADDRESS,
        set_error = sym set_stack_allocate_error,

);

global_asm!(
    "
    .global __chkstk_in_proc
    __chkstk_in_proc:
        /* Get the current stack pointer */
        lea r10, [rsp + 0x18]
        /* Calculate what the new stack pointer will be */
        sub r10, rax
        cmovb r10, r11
        mov r11, qword ptr gs:[0x0000000000000010]
        cmp r10, r11
        jae csip_ret
        and r10w,0x0F000
    csip_stackprobe:
        lea r11, [r11 + 0x0FFFFFFFFFFFFF000]
        mov byte ptr [r11], 0
        cmp r10, r11
        jne csip_stackprobe
    csip_ret:
        ret
",
);
