; Copyright 2024 The Hyperlight Authors.
;
; Licensed under the Apache License, Version 2.0 (the "License");
; you may not use this file except in compliance with the License.
; You may obtain a copy of the License at
;
;    http://www.apache.org/licenses/LICENSE-2.0
;
; Unless required by applicable law or agreed to in writing, software
; distributed under the License is distributed on an "AS IS" BASIS,
; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
; See the License for the specific language governing permissions and
; limitations under the License.

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