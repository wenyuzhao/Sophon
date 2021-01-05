ENTRY(_start)

SECTIONS {
    . = 0xffff000000080000;
    __kernel_start = .;
    
    .text : {
        *(.text._start)
        *(.text*)
    }
    
    .rodata : {
        *(.rodata*)
    }

    .data : {
        *(.data*)
    }

    .bss ALIGN(16): {
        __bss_start = .;
        *(.bss*);
        __bss_end = .;
    }

    __kernel_end = .;

    /DISCARD/ : {
        *(.comment) *(.gnu*) *(.note*) *(.eh_frame*)
    }
}