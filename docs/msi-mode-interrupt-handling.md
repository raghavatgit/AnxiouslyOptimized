# Message Signaled Interrupts (MSI) vs Line-Based IRQs

## The Pin Interrupt Bottleneck
Legacy line-based interrupts share physical IRQ pins, causing interrupt storms where the CPU must poll multiple drivers to identify the interrupt sender.

---

## MSI / MSI-X Architecture
Message Signaled Interrupts write target DWORDs directly into local APIC memory over the PCIe bus, assigning dedicated non-shared interrupt vectors to GPUs and NVMe drives.
