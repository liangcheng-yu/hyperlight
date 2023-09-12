ENTRY(entryPoint)

SECTIONS
{
    . = ALIGN(512);
    .text : 
    {
        *(.text)
    }
    . = ALIGN(512);
    .data : 
    {
        *(.data)
    }
    . = ALIGN(512);
    .bss : 
    {
        *(.bss)
    }
}
