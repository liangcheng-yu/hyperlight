
/// This is a dummy guest that allows us to test the hypervisor implementations without having to set up full host - > guest integration.
/// It is a simple program that just reads three arguments, checks if they are the expected values and then halts the CPU. 
/// We cannot use the Hyperlight test guests for this as the initialise function requires setup that we dont need to do unit test the hypervisor.
/// The purpose  is to ensure that we can run a binary that is compiled and linked the way that a real guest would be.
/// The only difference between the compilations for this and a real guest is that /GS is disabled for this guest so that we dont have to implement the stack cookie check.

#pragma optimize("", off)

void halt()
{
    const unsigned char hlt = 0xF4;
    ((void (*)()) & hlt)();
}

void mmio_read()
{
    const unsigned char mmio_read[4] = { 0x8a, 0x16, 0x00, 0x80 };
    ((void (*)()) & mmio_read)();
}

#pragma optimize("", on)
// This is the same entrypoint that the GuestLibrary provides.
__declspec(safebuffers) int entryPoint(long long a, long long b, int c)
{
    // Check that expected values were passed in
    if (a != 0x230000 || b != 1234567890 || c != 4096)
    {
        mmio_read();
    }
    halt(); 
    return 0;
}