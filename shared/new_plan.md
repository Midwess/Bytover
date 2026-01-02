# Potential Issues Causing Hangs During Multiple Concurrent Downloads

## Analysis of shared/src/entities/peer.rs

### Identified Concerns

1. **Lack of Concurrency Primitives**
   - The `Peer` struct is purely synchronous with no async/concurrent handling mechanisms
   - No mutexes, locks, or atomic operations visible in the entity layer
   - Issue: If multiple download sessions access or modify peer state concurrently, this could lead to race conditions or blocking

2. **Scope Filtering Operations**
   - Methods like `owned_scopes()` and `member_scopes()` iterate and filter on every call
   - No caching mechanism for scope lookups
   - Issue: During high-frequency concurrent downloads, repeated iterations could become a bottleneck

3. **Session Ownership Checks**
   - `is_owned()` and `is_member()` perform linear searches through scopes for each session check
   - O(n) complexity per check
   - Issue: With many concurrent sessions, these checks could accumulate and cause delays

4. **Missing Resource Limits**
   - No visible constraints on number of concurrent sessions per peer
   - No rate limiting or throttling mechanisms
   - Issue: Unlimited concurrent downloads could exhaust resources (file handles, memory, network connections)

5. **Potential Deadlock Scenarios**
   - If download logic uses shared state and multiple peers access the same resources
   - No timeout mechanisms visible in the entity definitions
   - Issue: Concurrent operations waiting for each other could deadlock

6. **Clone Operations**
   - `Peer` derives `Clone`, and `id()` method clones strings
   - Issue: Heavy cloning during concurrent operations could cause memory pressure and slow performance

7. **Error Handling Gaps**
   - `id()` uses `unwrap_or_default()` which could silently fail
   - Issue: Invalid peer IDs could cause sessions to hang waiting for non-existent peers

8. **Scope State Synchronization**
   - Multiple sessions might access the same `FindingScope`
   - No visible synchronization mechanism for scope state updates
   - Issue: Stale or inconsistent scope state across concurrent downloads

## Recommendations for Investigation

1. **Check the download implementation layer** - Look for async/await patterns and channel usage
2. **Review resource pooling** - Verify if connection pooling or file handle limits exist
3. **Add timeout mechanisms** - Ensure all network operations have proper timeouts
4. **Implement concurrency limits** - Add semaphores or rate limiting for concurrent downloads
5. **Profile memory usage** - Check if cloning and large data structures cause memory issues
6. **Add observability** - Instrument the code to track where hangs occur
7. **Review TransferSession logic** - Investigate how sessions are managed concurrently
8. **Check matchbox_protocol usage** - Verify the underlying P2P protocol handles concurrency correctly
