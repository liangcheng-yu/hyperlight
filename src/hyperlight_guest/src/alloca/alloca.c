#include <stddef.h>
#include <stdint.h>

void* _alloca_wrapper(size_t size) {
    uint8_t vla[size];

    // initialize w/ 0
    for (size_t i = 0; i < size; i++) {
        vla[i] = 0;
    }
    
    return (void*) &vla[0];
}