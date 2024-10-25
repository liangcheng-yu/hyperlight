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

; sets up the security cookie used by /GS compiler option and checks value is valid
; calls report_gsfailure if value is invalid

_TEXT  SEGMENT
extern pPeb: qword
extern __security_cookie: qword
extern report_gsfailure : proc
__security_check_cookie PROC
    cmp         rcx,qword ptr [__security_cookie]
    jne         report_gsfailure
    ret
__security_check_cookie ENDP

__security_init_cookie PROC
    sub         rsp,10h                             ; make space on the stack to save r10
    mov         qword ptr [rsp],r10
    mov         r10,qword ptr [pPeb]                ; get the cookie init value provided by the host in the peb.
    mov         r10,qword ptr [r10]                
    xor         r10,rdx                             ; xor with the seed passed to entrypoint
    and         rdx,0
    mov         qword ptr [__security_cookie],r10
    mov         r10,qword ptr [rsp]                 ; restore r10
    add         rsp,10h
    ret
__security_init_cookie ENDP

; TODO: Does this need to do anything? I think it may have been provided to support _set_security_error_handler which is no longer allowed
; compiler complains if it is not declated but I cant find anywhere it is called. in libcmt the last thing it does is jmp to __security_check_cookie
; so that is all this implementation does.
__GSHandlerCheck PROC
    jmp         __security_check_cookie
__GSHandlerCheck ENDP

_TEXT  ENDS

END