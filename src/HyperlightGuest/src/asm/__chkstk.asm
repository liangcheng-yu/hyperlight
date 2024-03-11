; implements __chkstk to check if stack overflow will occur
; __chkstack function is called by function prolog when the local variables/alloca space required by the function exceeds the value specified in 
; the /Gs compiler flag see https://docs.microsoft.com/en-us/cpp/build/reference/gs-control-stack-checking-calls (despite what the docs say the 
; default check seems to happen if storage required is >4K .)
; this functions calculates what the top of the stack would be by adding the amount of space the function being called requires to the current stack pointer
; if this is beyond the minStackAddrress (which is set by the host) then calling the function would cause a stack overflow
; and it calls seterror with error code 9 (stack overflow)
; 
; TODO:
; NOTE that this custom check has no effect when running in memory as the min stack address would need to be set each time the guest was invoked
; as the stack is provided by the host process, so in this case it calls the same implmentation that is included in libcmt.lib

_TEXT  SEGMENT

extern setError : proc
extern pPeb: qword
extern runningInHyperlight: byte

__chkstk PROC
    sub         rsp,10h                             ; make space on the stack to save r10,r11
    mov         qword ptr [rsp],r10
    mov         qword ptr [rsp+8],r11
    xor         r11,r11
    movzx       r11,byte ptr [runningInHyperlight]    ; check if we are running in Hyperlight , do the inproc check
    test        r11,r11
    je          call_chk_inproc
    lea         r10,[rsp+18h]                       ; get the current stack address
    sub         r10,rax                             ; calculate the new stack address
    mov         r11,qword ptr [pPeb]                ; get the minimum allowed stack address 
    mov         r11,qword ptr [r11+90h]             ; Make sure to update this address if the PEB structure changes, 
                                                    ; the value added to R11 regsiter here is the number of bytes offset from the start of the PEB to the adress of the stack. 
                                                    ; hence if there are any fields added in the PEB this needs to be updated. See the comments in layout.rs for an explanation of the layout and fields
    cmp         r10,r11                             ; check if this allocation would exceed top of stack.
    jae         csret
    xor         edx,edx                             ; if the allocation would call stack overflow set the parameters for set error and call it
    mov         ecx,9                               ; guest error code 9 is stack overflow see hyperlight_error.h
    call        setError                            ; TODO add some context to the message
call_chk_inproc:
    call        __chkstk_in_proc 
csret:
    mov         r10,qword ptr [rsp]
    mov         r11,qword ptr [rsp+8]
    add         rsp,10h
    ret
__chkstk ENDP


__chkstk_in_proc PROC
    xor         r11,r11
    lea         r10,[rsp+18h]
    sub         r10,rax
    cmovb       r10,r11
    mov         r11,qword ptr gs:[0000000000000010h]
    cmp         r10,r11
    jae         cs20
    and         r10w,0F000h
cs10:
    lea         r11,[r11+0FFFFFFFFFFFFF000h]
    mov         byte ptr [r11],0
    cmp         r10,r11
    jne         cs10
cs20:
    ret
__chkstk_in_proc ENDP

_TEXT   ENDS

END