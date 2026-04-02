#####
## QEMU
#####
QEMU = qemu-system-riscv64
MACH = virt
CPU = rv64
CPUS = 4
MEM = 128M
TARGET = target/riscv64gc-unknown-none-elf/debug/learn_os_riscv

#####
## BUILD & RUN
#####
all:
	cargo build

run: all
	$(QEMU) -machine $(MACH) -cpu $(CPU) -smp $(CPUS) -m $(MEM) \
		-nographic -serial mon:stdio -bios none -kernel $(TARGET)

#####
## TEST (chạy trên QEMU với sifive_test device để auto-exit)
#####
test:
	cargo test --no-run 2>&1
	$(QEMU) -machine $(MACH) -cpu $(CPU) -smp $(CPUS) -m $(MEM) \
		-nographic -serial mon:stdio -bios none \
		-kernel $$(cargo test --no-run --message-format=json 2>/dev/null \
			| python3 -c "import sys,json; [print(l['executable']) for l in (json.loads(line) for line in sys.stdin) if l.get('executable')]" \
			| head -1)

.PHONY: all run test clean
clean:
	cargo clean