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
    
;takes a port number and a byte as an argument and executes the out instruction.

hloutb PROC
    mov al,dl
    mov dx,cx
    out dx,al
    ret
hloutb ENDP

_TEXT   ENDS

END