TARGET := riscv64gc-unknown-none-elf
PROFILE := release
KERNEL := ../target/$(TARGET)/$(PROFILE)/rvos
LOG := INFO
QEMU_ARGS :=
SMP ?= 1
MEMORY_SIZE := 1024M


ifeq ($(GUI),y)
	QEMU_ARGS += -device virtio-gpu-device
	FEATURES += gui
else
	QEMU_ARGS += -nographic
endif


all: run

build:
	@echo "Building..."
	@LOG=$(LOG) cargo build --release

run: build
	qemu-system-riscv64 \
            -M virt \
            -bios default \
            -kernel $(KERNEL)\
            -$(QEMU_ARGS) \
            -smp $(SMP) -m $(MEMORY_SIZE) \
            -serial mon:stdio
	@-rm $(IMG)


kernel_asm:
	@riscv64-unknown-elf-objdump -d $(KERNEL) > kernel.asm
	@vim kernel.asm
	@rm kernel.asm