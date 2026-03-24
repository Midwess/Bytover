# Performance Enhancement Recommendations

This document outlines micro-optimization opportunities for the RPC Signalling codebase. These are small, targeted improvements that can enhance performance without changing the application flow.

## Table of Contents
- [1. Memory Allocation Optimizations](#1-memory-allocation-optimizations)
- [2. String & Clone Optimizations](#2-string--clone-optimizations)
- [3. Data Structure Optimizations](#3-data-structure-optimizations)
- [4. Algorithm Optimizations](#4-algorithm-optimizations)
- [5. Compiler & Build Optimizations](#5-compiler--build-optimizations)

---

## 1. Memory Allocation Optimizations

### 1.1 Pre-allocate HashMap Capacity
**File:** `app/src/signaller.rs:324`

**Current:**
```rust
let mut scope_messages: HashMap<String, Vec<SignallingMessage>> = HashMap::new();
```

**Optimization:**
```rust
let mut scope_messages: HashMap<String, Vec<SignallingMessage>> = HashMap::with_capacity(broadcast_requests.len());
```

**Benefit:** Reduces rehashing during insertion when we know approximately how many entries we'll have.

---

### 1.2 Reduce Channel Buffer Size
**File:** `app/src/signaller.rs:250`

**Current:**
```rust
let (scope_request_tx, scope_request_rx) = mpsc::channel(128 * 1024);
```

**Optimization:**
```rust
let (scope_request_tx, scope_request_rx) = mpsc::channel(8192);
```

**Benefit:** 128K buffer (131,072 messages) is likely excessive. Start with 8K and adjust based on actual metrics. This reduces memory footprint.

---

### 1.3 Optimize Temporary IP Address Storage
**File:** `app/src/websocket.rs:313-314`

**Current:**
```rust
let ip_address = Arc::new(Mutex::new(peer_addr.clone()));
let ip_address_clone = ip_address.clone();
```

**Optimization:**
```rust
use std::sync::OnceLock;
let ip_address = Arc::new(OnceLock::new());
let _ = ip_address.set(peer_addr.clone());
let ip_address_clone = ip_address.clone();
```

**Benefit:** `OnceLock` is more efficient than `Mutex<String>` when the value is set once and read many times. No lock contention.

---

## 2. String & Clone Optimizations

### 2.1 Remove Redundant Assignment
**File:** `app/src/signaller.rs:199-201`

**Current:**
```rust
pub async fn broadcast_to_all(&self, message: SignallingMessage) {
    let mut from_scope = message.from_scope.clone();
    from_scope = Some(self.id.clone());
```

**Optimization:**
```rust
pub async fn broadcast_to_all(&self, message: SignallingMessage) {
    let from_scope = Some(self.id.clone());
```

**Benefit:** Eliminates unnecessary clone of `message.from_scope` since it's immediately overwritten.

---

### 2.2 Avoid String Allocation in ID Generator
**File:** `app/src/main.rs:15`

**Current:**
```rust
init_scoped_id_generator("rpc-signalling".to_string());
```

**Optimization:**
```rust
init_scoped_id_generator("rpc-signalling");
```

**Benefit:** If the function accepts `impl Into<String>` or `&str`, avoid the explicit `.to_string()` call. Let the compiler optimize.

---

### 2.3 Reuse String Buffer for Encoding
**File:** `app/src/websocket.rs:229-230`

**Current:**
```rust
let mut buf = Vec::new();
let encoded = message.encode(&mut buf);
```

**Optimization:** Store a reusable buffer in the Client struct:
```rust
// In Client struct
encode_buffer: Mutex<Vec<u8>>,

// In send method
let mut buf = self.encode_buffer.lock().await;
buf.clear();
let encoded = message.encode(&mut *buf);
```

**Benefit:** Reduces allocations by reusing the same buffer for encoding messages.

---

### 2.4 Optimize HashMap Drain Strategy
**File:** `app/src/websocket.rs:214-216`

**Current:**
```rust
if queue_guard.len() > 200 {
    queue_guard.drain();
}
```

**Optimization:**
```rust
if queue_guard.len() > 200 {
    // Keep only the most recent 100 entries
    queue_guard.retain(|_, v| v.load(std::sync::atomic::Ordering::Relaxed) > threshold);
    // Or use a more sophisticated LRU eviction
}
```

**Benefit:** Instead of removing all entries when limit is reached, selectively remove old entries. This prevents cache thrashing.

---

## 3. Data Structure Optimizations

### 3.1 Use SmallVec for Small Collections
**File:** `app/src/websocket.rs:71` and `app/src/locator.rs:43`

**Current:**
```rust
let request_scopes = message.scopes.iter().map(|it| Scope::new(it)).collect::<Vec<_>>();
```

**Optimization:**
```rust
use smallvec::SmallVec;
let request_scopes: SmallVec<[Scope; 4]> = message.scopes.iter().map(|it| Scope::new(it)).collect();
```

**Benefit:** Most clients join 1-4 scopes. SmallVec stores up to 4 elements inline without heap allocation.

**Note:** Add `smallvec = "1.11"` to Cargo.toml

---

### 3.2 Use FxHashMap for Non-Cryptographic Hashing
**File:** Multiple files using `HashMap`

**Current:**
```rust
use std::collections::HashMap;
```

**Optimization:**
```rust
use rustc_hash::FxHashMap as HashMap;
```

**Benefit:** FxHashMap is faster than the default SipHash for non-cryptographic use cases (internal data structures).

**Note:** Add `rustc-hash = "2.0"` to Cargo.toml

---

### 3.3 Reduce VecDeque Initial Capacity
**File:** `app/src/signaller.rs:263`

**Current:**
```rust
let mut broadcast_requests: VecDeque<(String, SignallingMessage)> = VecDeque::with_capacity(4096);
```

**Optimization:**
```rust
let mut broadcast_requests: VecDeque<(String, SignallingMessage)> = VecDeque::with_capacity(512);
```

**Benefit:** 4096 is likely oversized for most scenarios. Start smaller and let it grow if needed.

---

## 4. Algorithm Optimizations

### 4.1 Deduplicate Server Selection Logic
**File:** `app/src/turn_manager.rs:333-375`

**Current:** `select_stun_for_peer` and `select_turn_for_peer` contain identical logic.

**Optimization:**
```rust
fn select_server_for_peer(&self, peer_continent: Continent, servers: &[TurnServer]) -> TurnServer {
    let priority_order = peer_continent.priority_order();

    for &target_continent in priority_order {
        let candidates: Vec<&TurnServer> = servers.iter()
            .filter(|s| s.continent == target_continent)
            .collect();

        if !candidates.is_empty() {
            return (*candidates.iter()
                .min_by_key(|s| s.counter.load(Ordering::Relaxed))
                .unwrap())
                .clone();
        }
    }

    servers.iter()
        .min_by_key(|s| s.counter.load(Ordering::Relaxed))
        .unwrap()
        .clone()
}
```

**Benefit:** Reduces code duplication and makes the logic easier to maintain. The compiler will inline it anyway.

---

### 4.2 Avoid Unnecessary Vec Collection
**File:** `app/src/turn_manager.rs:280-281`

**Current:**
```rust
let servers = self.discovered_servers.lock().await;
let servers_vec: Vec<TurnServer> = servers.iter().cloned().collect();
drop(servers);
```

**Optimization:**
```rust
let servers = self.discovered_servers.lock().await;
// Clone only the servers we need for selection
let server_count = servers.len();
let servers_snapshot: Box<[TurnServer]> = servers.iter().cloned().collect();
drop(servers);
```

**Benefit:** `Box<[TurnServer]>` is slightly more memory-efficient than `Vec<TurnServer>` when we don't need the extra capacity.

---

### 4.3 Cache Parsed Header Values
**File:** `app/src/websocket.rs:246-266`

**Current:** Header parsing happens on every connection.

**Optimization:** The extraction logic is already optimal (early returns). However, consider:
```rust
// Use lazy evaluation only when needed
fn extract_ip_from_request(req: &Request, peer_addr: &str) -> String {
    req.headers()
        .get("CF-Connecting-IP")
        .or_else(|| req.headers().get("X-Forwarded-For"))
        .or_else(|| req.headers().get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap().trim().to_string())
        .unwrap_or_else(|| peer_addr.to_string())
}
```

**Benefit:** More functional style, potentially clearer intent. Compiler should generate similar code.

---

### 4.4 Optimize Scope Message Batching
**File:** `app/src/signaller.rs:324-341`

**Current:** Messages are collected into HashMap then processed.

**Optimization:** Consider using `group_by` pattern to avoid intermediate allocation:
```rust
// Sort by scope_id first if possible
broadcast_requests.make_contiguous().sort_by(|a, b| a.0.cmp(&b.0));

let mut current_scope: Option<String> = None;
let mut current_messages: Vec<SignallingMessage> = Vec::new();

for (scope_id, message) in std::mem::take(&mut broadcast_requests) {
    // Process batched messages when scope changes
}
```

**Benefit:** Avoids HashMap allocation for message grouping. Only beneficial if scope_id locality is high.

---

## 5. Compiler & Build Optimizations

### 5.1 Enable Link-Time Optimization (LTO)
**File:** `Cargo.toml`

**Add to workspace:**
```toml
[profile.release]
lto = "thin"
codegen-units = 1
opt-level = 3
strip = true
```

**Benefit:**
- LTO enables cross-crate optimizations
- `codegen-units = 1` allows better optimization at cost of compile time
- `strip = true` reduces binary size

---

### 5.2 Use CPU-Specific Optimizations
**Build command:**
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

**Benefit:** Enables CPU-specific SIMD instructions and optimizations for the target hardware.

---

### 5.3 Profile-Guided Optimization (PGO)
**Process:**
1. Build with instrumentation: `RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" cargo build --release`
2. Run typical workload to collect profiles
3. Rebuild with profile data: `RUSTFLAGS="-Cprofile-use=/tmp/pgo-data/merged.profdata" cargo build --release`

**Benefit:** Compiler optimizes based on actual runtime behavior.

---

### 5.4 Enable Additional Optimizations for Dependencies
**File:** `Cargo.toml`

**Add:**
```toml
[profile.dev.package."*"]
opt-level = 2
```

**Benefit:** Optimizes dependencies in debug builds, improving development performance without slowing down your code's compile time.

---

## 6. Additional Micro-Optimizations

### 6.1 Use `Arc::clone()` Instead of `.clone()`
**Multiple files**

**Current:**
```rust
let turn_manager_clone = turn_manager.clone();
```

**Optimization:**
```rust
let turn_manager_clone = Arc::clone(&turn_manager);
```

**Benefit:** More explicit intent. Performance is identical, but clarity is improved.

---

### 6.2 Inline Small Functions
**File:** `app/src/websocket.rs:157-163`

**Current:**
```rust
pub fn id(&self) -> String {
    self.socket_id.clone()
}
```

**Optimization:**
```rust
#[inline]
pub fn id(&self) -> String {
    self.socket_id.clone()
}
```

**Benefit:** Compiler hint to inline hot-path functions. May reduce call overhead.

---

### 6.3 Use `if let` Instead of Pattern Match Where Appropriate
**File:** `app/src/locator.rs:75`

**Current:**
```rust
neighbors_with_angles.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
```

**Optimization:**
```rust
neighbors_with_angles.sort_by(|a, b| {
    a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
});
```

**Benefit:** Already optimal. This is fine as-is.

---

## Summary of Impact

| Optimization Category | Estimated Impact | Implementation Effort |
|----------------------|------------------|---------------------|
| Memory Allocations | Medium (5-10% memory reduction) | Low |
| String & Clone | Low-Medium (2-5% CPU reduction) | Low |
| Data Structures | Medium (10-15% for hot paths) | Medium |
| Algorithm | Low (1-3% improvement) | Low |
| Compiler Optimizations | High (15-25% throughput) | Low |

## Measurement & Validation

Before and after implementing these optimizations:

1. **Benchmark with `criterion`**: Add micro-benchmarks for hot paths
2. **Profile with `perf`**: `perf record -g cargo run --release`
3. **Memory profiling**: Use `valgrind --tool=massif` or `heaptrack`
4. **Monitor metrics**: Track latency (p50, p95, p99) and throughput

## Implementation Priority

**High Priority (Implement First):**
1. Compiler & Build Optimizations (5.1-5.4)
2. FxHashMap replacement (3.2)
3. Remove redundant assignments (2.1, 2.4)

**Medium Priority:**
1. Pre-allocate capacities (1.1, 1.3)
2. Deduplicate logic (4.1)
3. Reduce initial capacities (1.2, 3.3)

**Low Priority (Monitor first):**
1. SmallVec usage (3.1)
2. Reusable buffers (2.3)
3. Inline hints (6.2)

---

**Note:** Always measure before and after optimization. Some optimizations may have negligible impact in practice depending on actual usage patterns.
