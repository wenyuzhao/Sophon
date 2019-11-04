ENTRY(_start)

SECTIONS {
    . = 0xffff000000080000;
    
    .text : {
        *(.text._start) *(.text*)
    }
    
    .rodata : {
        *(.rodata*)
    }

    .data : {
        *(.data*)
    }

    .bss ALIGN(8): {
        __bss_start = .;
        *(.bss*);
        __bss_end = .;
    }

    /DISCARD/ : { *(.comment*) }
}