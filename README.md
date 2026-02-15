# Secex.rs

In development

## What?

Rust project to launch Firecracker MicroVM's

## Why?

Potentially run untrusted or AI generated code in isolation

## Download kernel

Needed for the VM

```bash
make download-kernel

```

## Development

### 1. Build init program for the VM

```bash
make build-init
```

### 2. Build base rootfs image for the VM

```bash
make build-rootfs
```

### 3. Run the VM orchestrator

```bash
make orchestrator
```
