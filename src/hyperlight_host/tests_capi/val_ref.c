#include "hyperlight_host.h"
#include "stdint.h"
#include "stdlib.h"
#include "val_ref.h"
#include "mem.h"

// create a dummy Val with the given size.
// you must free the returned Val with handle_free
// when you're done.
struct Val *dummy_val_ref(unsigned int size)
{
    int8_t *data = malloc(sizeof(int8_t) * size);
    for (size_t i = 0; i < size; i++)
    {
        data[i] = 0;
    }
    struct Val *ret = val_ref_new(data, size, Raw);
    free(data);
    return ret;
}

struct Val *val_ref_empty()
{
    int8_t *mem = create_i8_mem(0, false);
    struct Val *val_ref = val_ref_new(mem, 0, Raw);
    free(mem);
    return val_ref;
}
