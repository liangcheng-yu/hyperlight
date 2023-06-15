#pragma once
#include "hyperlight_error.h"
#include "guest_error_builder.h"
#include "guest_error_reader.h"
#include "guest_function_details_builder.h"
#include "guest_function_details_reader.h"

// dlmalloc defines

#define MAX_SIZE_T           (~(size_t)0)
#define MFAIL                ((void*)(MAX_SIZE_T))

// Stack:<reserve> should all be set to the desired stack size value when building/linking this guest
// The value should be specified in the GUEST_STACK_SIZE build parameter (the commit size will also be set to the same value but it is ignored at runtime)
// this value is used to configure the stack size for the sandbox.

#ifndef GUEST_STACK_SIZE
#pragma message("GUEST_STACK_SIZE is not defined.")
#define GUEST_STACK_SIZE 32768
#endif
void HyperlightMainDefault() {}

// If the guest does not define a GuestInitFunction use the empty GuestInitDefault instead see https://devblogs.microsoft.com/oldnewthing/20200731-00/?p=104024

#pragma comment(linker, "/alternatename:HyperlightMain=HyperlightMainDefault")  

uint8_t* GuestDispatchFunctionDefault(ns(FunctionCall_table_t) functionCall)
{
    char * functionName = (char *)ns(FunctionCall_function_name(functionCall));
    setError(GUEST_FUNCTION_NOT_FOUND, functionName);
    return 0;
}

// If the guest does not define a GuestDispatchFunction use the default GuestDispatchFunctionDefault instead

#pragma comment(linker, "/alternatename:GuestDispatchFunction=GuestDispatchFunctionDefault")  

uint64_t getrsi();
uint64_t getrdi();
void setrsi(uint64_t rsi);
void setrdi(uint64_t rdi);
void hloutb(uint16_t port, uint8_t value);

#define OUTB_LOG 99
#define OUTB_CALL_FUNCTION 101
#define OUTB_ABORT 102

void __security_init_cookie(void);