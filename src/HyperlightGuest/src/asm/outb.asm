_TEXT  SEGMENT
    
; outb output to port for us in Hyperlight when running in Hypervisor.

hvoutb PROC
    mov dword ptr [eax],edx
    out dx, al
    ret
hvoutb ENDP

_TEXT   ENDS

END