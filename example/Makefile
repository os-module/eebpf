# Building
ARCH := x86_64
QEMU_EXEC :=
GDB ?= n

ifeq ($(ARCH), x86_64)
  TARGET := x86_64-unknown-none
  KERNEL_ELF := ../target/$(TARGET)/release/example
  KERNEL_BIN := $(KERNEL_ELF).bin
  QEMU_EXEC += qemu-system-x86_64 \
				-machine q35 \
				-enable-kvm \
				-kernel $(KERNEL_ELF) \
				-cpu IvyBridge-v2
  BUS := pci
else ifeq ($(ARCH), riscv64)
  TARGET := riscv64gc-unknown-none-elf
  KERNEL_ELF := ../target/$(TARGET)/release/example
  KERNEL_BIN := $(KERNEL_ELF).bin
  QEMU_EXEC += qemu-system-$(ARCH) \
				-machine virt \
				-kernel $(KERNEL_BIN)
else ifeq ($(ARCH), aarch64)
  TARGET := aarch64-unknown-none-softfloat
  KERNEL_ELF := ../target/$(TARGET)/release/example
  KERNEL_BIN := $(KERNEL_ELF).bin  
  QEMU_EXEC += qemu-system-$(ARCH) \
				-cpu cortex-a72 \
				-machine virt \
				-kernel $(KERNEL_BIN)
else ifeq ($(ARCH), loongarch64)
  TARGET := loongarch64-unknown-none
  KERNEL_ELF := ../target/$(TARGET)/release/example
  KERNEL_BIN := $(KERNEL_ELF).bin
  QEMU_EXEC += qemu-system-$(ARCH) -kernel $(KERNEL_ELF)
  BUILD_ARGS += -Z build-std=core,alloc
  BUS := pci
else
  $(error "ARCH" must be one of "x86_64", "riscv64", "aarch64" or "loongarch64")
endif

# Binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

build: env $(KERNEL_BIN) 

env:
	(rustup target list | grep "$(TARGET) (installed)") || rustup target add $(TARGET)
	cargo install cargo-binutils
	rustup component add rust-src
	rustup component add llvm-tools-preview

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

kernel:
	@echo Platform: $(BOARD)
	@cargo build $(BUILD_ARGS) --release --target $(TARGET)

clean:
	@cargo clean

run: run-inner

QEMU_EXEC += -nographic -smp 1
ifeq ($(GDB), y)
  QEMU_EXEC += -s -S
endif

run-inner: build
	$(QEMU_EXEC)
# grep "\[kernel\] Hello, world!" output.log

gdb-server:
	$(QEMU_EXEC) -s -S

gdb-client:
	gdb -ex "file $(KERNEL_ELF)" -ex "target remote :1234"

test:
	make ARCH=aarch64 run
	make ARCH=riscv64 run
	make ARCH=x86_64 run
	make ARCH=loongarch64 run


asm:
	objdump -d $(KERNEL_ELF) > kernel.asm
	vim kernel.asm
	rm kernel.asm
.PHONY: build env kernel clean run-inner
