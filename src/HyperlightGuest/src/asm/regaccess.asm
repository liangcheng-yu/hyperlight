_TEXT  SEGMENT
    
; Functions used to save/restore both rsi and rdi to allow
; our windows exe to run under Linux when it has to call out
; to a Linux host when running 'in memory' and the Linux host
; is not preserving rsi/rdi like the MSVC compiler expects.

getrsi PROC
    mov rax, rsi
    ret
getrsi ENDP

setrsi PROC
    mov rsi, rcx
    ret
setrsi ENDP

getrdi PROC
    mov rax, rdi
    ret
getrdi ENDP

setrdi PROC
    mov rdi, rcx
    ret
setrdi ENDP

_TEXT   ENDS

END