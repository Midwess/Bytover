## Transfer Session Display Improvements

### Overview
Enhance the P2P transfer session UI to properly display download progress, speed metrics, and per-resource completion status.

### Tasks

#### 1. Fix Transfer Session Progress Calculation
**File:** `entities/transfer_session.rs`

**Issue:** The overall progress for P2P transfer sessions is stuck at 0%

**Required Changes:**
- Update the progress calculation logic to aggregate progress from all transfer resources
- Ensure the progress value is computed as: `(completed_bytes / total_bytes) * 100`
- Update the progress field whenever resource progress changes
- Verify progress is correctly propagated to the frontend

#### 2. Add Download Speed Display
**File:** `receive_board.tsx` (note: check spelling - might be `receive_board.tsx`)

**Required Changes:**
- Add a download speed indicator component
- Calculate speed as bytes transferred per second
- Display speed in appropriate units (KB/s, MB/s)
- Update speed metric in real-time (suggested interval: every 500ms-1s)
- Position the speed display prominently in the transfer UI

#### 3. Implement Per-Resource Progress Display
**File:** Transfer session resources component

**Required Changes:**
- Display individual completion progress for each resource item
- Show progress bar or percentage for each file/resource
- Include resource name and size
- Highlight currently transferring resources
- Show completed/pending status clearly

### Success Criteria
- [ ] Overall session progress shows accurate percentage (not stuck at 0%)
- [ ] Download speed is visible and updates in real-time
- [ ] Each resource shows its individual progress
- [ ] UI updates smoothly without performance issues