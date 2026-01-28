# Plano de Implementação: OOM Guard

## Visão Geral
Daemon de gerenciamento de memória em Rust que monitora o sistema e mata processos preventivamente antes do OOM killer do kernel.

## Objetivos
- ✅ Monitorar memória RAM e swap em tempo real
- ✅ Matar processos com base em thresholds configuráveis
- ✅ Suportar execução como serviço systemd
- ✅ Configuração via **CLI arguments** e **environment variables**
- ✅ **Notificações D-Bus** e **hooks de scripts** personalizáveis
- ✅ Modo dry-run para testes

## Arquitetura

### Estrutura de Módulos

```
src/
├── main.rs              # Entry point, CLI parsing
├── lib.rs               # Library exports
├── config/
│   ├── mod.rs          # Configuration module
│   ├── args.rs         # CLI arguments parsing
│   └── env.rs          # Environment variables parsing
├── monitor/
│   ├── mod.rs          # Memory monitoring
│   ├── meminfo.rs      # /proc/meminfo parser
│   └── process.rs      # Process information
├── killer/
│   ├── mod.rs          # Process killing logic
│   ├── selector.rs     # Process selection
│   └── signals.rs      # Signal handling
├── daemon/
│   ├── mod.rs          # Daemon lifecycle
│   └── service.rs      # Main monitoring loop
└── notify/
    ├── mod.rs          # Notifications
    └── hooks.rs        # Script hooks
```

## Fases de Implementação

### Fase 1: Estrutura Base e Configuração
**Arquivos:**
- `Cargo.toml` - Dependências
- `src/lib.rs` - Exports da biblioteca
- `src/config/mod.rs` - Módulo de configuração
- `src/config/args.rs` - Parser de argumentos CLI

**Dependências:**
- `clap` (v4) - Parsing de CLI arguments
- `anyhow` - Error handling
- `log` e `env_logger` - Logging
- `procfs` - Leitura eficiente de /proc
- `nix` - Unix system calls e sinais
- `regex` - Para filtros --prefer/--avoid/--ignore

### Fase 2: Monitor de Memória
**Arquivos:**
- `src/monitor/mod.rs`
- `src/monitor/meminfo.rs` - Parser de `/proc/meminfo`
- `src/monitor/process.rs` - Leitura de `/proc/[pid]/*`

### Fase 3: Seletor de Processos
**Arquivos:**
- `src/killer/mod.rs`
- `src/killer/selector.rs`

### Fase 4: Gerenciamento de Sinais
**Arquivos:**
- `src/killer/signals.rs`

### Fase 5: Loop Principal do Daemon
**Arquivos:**
- `src/daemon/mod.rs`
- `src/daemon/service.rs`

### Fase 6: Sistema de Notificações
**Arquivos:**
- `src/notify/mod.rs`
- `src/notify/hooks.rs`

### Fase 7: Integração Systemd
**Arquivos:**
- `systemd/oom_guard.service`
- `install.sh`
- `README.md`

### Fase 8: Recursos Avançados
- `--dryrun` - Simular sem matar processos
- `-p` - Ajustar niceness e oom_score_adj do próprio daemon
- Modo debug com verbose logging

## Estruturas de Dados Principais

### Config
```rust
pub struct Config {
    // Memory thresholds
    pub mem_threshold_warn: f64,
    pub mem_threshold_kill: f64,
    pub swap_threshold_warn: f64,
    pub swap_threshold_kill: f64,

    // Process selection
    pub prefer_regex: Vec<String>,
    pub avoid_regex: Vec<String>,
    pub ignore_regex: Vec<String>,
    pub sort_by_rss: bool,

    // Behavior
    pub kill_process_group: bool,
    pub report_interval: Option<u64>,
    pub dry_run: bool,

    // Hooks
    pub pre_kill_script: Option<String>,
    pub post_kill_script: Option<String>,
    pub notify_dbus: bool,
}
```

### MemInfo
```rust
pub struct MemInfo {
    pub mem_total: u64,      // KiB
    pub mem_available: u64,  // KiB
    pub swap_total: u64,     // KiB
    pub swap_free: u64,      // KiB
}
```

### ProcessInfo
```rust
pub struct ProcessInfo {
    pub pid: i32,
    pub name: String,
    pub oom_score: i32,
    pub oom_score_adj: i32,
    pub vm_rss: u64,         // KiB
    pub is_kernel_thread: bool,
}
```

## CLI Flags

### Obrigatórias
- `-m PERCENT[,KILL_PERCENT]` - Memory threshold (warn, kill)
- `-s PERCENT[,KILL_PERCENT]` - Swap threshold (warn, kill)
- `-M SIZE[,KILL_SIZE]` - Memory threshold em KiB
- `-S SIZE[,KILL_SIZE]` - Swap threshold em KiB
- `-n` - Enviar notificações D-Bus
- `-N /path/to/script` - Post-kill script
- `-P /path/to/script` - Pre-kill script
- `-g` - Kill process group inteiro
- `-r INTERVAL` - Report interval em segundos
- `-p` - Set niceness e oom_score_adj
- `--prefer REGEX` - Preferir matar processos matching
- `--avoid REGEX` - Evitar matar processos matching
- `--ignore REGEX` - Ignorar processos matching completamente
- `--dryrun` - Dry run mode
- `--sort-by-rss` - Ordenar por RSS ao invés de oom_score
- `-h, --help` - Mostrar help
- `-v, --version` - Mostrar versão
- `-d` - Enable debug output

### Environment Variables
- `OOM_GUARD_MEM_THRESHOLD`
- `OOM_GUARD_SWAP_THRESHOLD`
- `OOM_GUARD_PREFER`
- `OOM_GUARD_AVOID`
- `OOM_GUARD_NOTIFY`
- `OOM_GUARD_DRY_RUN`

## Systemd Service Unit

```ini
[Unit]
Description=OOM Guard - Memory Monitor Daemon
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/oom_guard -m 10,5 -s 10,5 -n -r 3600
Restart=always
RestartSec=10

# Security hardening
CapabilityBoundingSet=CAP_KILL CAP_DAC_OVERRIDE CAP_SYS_NICE CAP_SYS_PTRACE
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/proc

[Install]
WantedBy=multi-user.target
```

## Status: IMPLEMENTADO

Todas as fases foram implementadas com sucesso.
