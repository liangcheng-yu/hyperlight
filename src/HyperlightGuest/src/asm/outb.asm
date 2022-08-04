_TEXT  SEGMENT
    
;takes a port number and a byte as an argument and executes the out instruction.

hloutb PROC
    mov al,dl
    mov dx,cx
    out dx,al
    ret
hloutb ENDP

_TEXT   ENDS

END