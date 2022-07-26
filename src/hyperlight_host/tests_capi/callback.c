#include "hyperlight_host.h"
#include "val_ref.h"
#include "callback.h"

struct Val *test_callback(struct Val *val)
{
    val_ref_free(val);
    return dummy_val_ref(10);
}
