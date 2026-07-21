# Synthesize IDE: Production Readiness Test Plan

## Objective
Validate that all orchestration, context operating system, multi-role coordination, and dream mode features work end-to-end with a real language model (Qwen 2.5 Coder 32B Q5).

## Test Infrastructure

### Prerequisites
- ✅ Rust workspace compiles with 0 warnings
- ✅ 132 unit/integration tests passing
- ✅ TypeScript frontend passes typecheck and tests
- ✅ Qwen 2.5 Coder 32B Q5 model downloaded (20GB)
- ✅ llama.cpp compiled with GPU support
- ✅ llama-server running on localhost:8000

---

## Phase 1: Model Runtime Integration (Tier 1: Critical)

### Test 1.1: llama.cpp Server Health
- **Goal**: Verify model endpoint is responding
- **Steps**:
  1. Start llama.cpp with Qwen model
  2. Call `GET http://localhost:8000/v1/models`
  3. Verify response includes model name and context window
- **Pass Criteria**: HTTP 200, model metadata returned
- **Timeout**: 30 seconds
- **Automated**: ✅ Can script this

### Test 1.2: Runtime Capability Registration
- **Goal**: IDE registers the model's capabilities correctly
- **Steps**:
  1. Open IDE
  2. Go to Settings → Runtime Configuration
  3. Configure: Provider=Local, Endpoint=http://localhost:8000/v1, Model=qwen2.5-coder-32b
  4. Click "Validate & Save"
  5. Check database for capability record
- **Pass Criteria**: Capability persisted with correct token window
- **Token Window Expected**: ~32k (depends on model config)
- **Safety Margin**: 512 tokens reserved
- **Maximum Output**: 2048 tokens
- **Automated**: ✅ Via Tauri commands

### Test 1.3: Context Capsule Compilation with Real Model
- **Goal**: Verify context OS compiles valid capsules for the registered model
- **Steps**:
  1. Create test initiative
  2. Add mock objective
  3. Invoke Dreamer role with small prompt
  4. Verify context capsule is compiled (not rejected for overflow)
  5. Check capsule includes correct token budget
- **Pass Criteria**: Capsule compiled, tokens ≤ (window - output - safety)
- **Automated**: ✅ Via orchestration-core tests + IDE

---

## Phase 2: Role Invocation with Real Model (Tier 1: Critical)

### Test 2.1: Dreamer Role - Opportunity Generation
- **Goal**: Dreamer generates realistic opportunity hypotheses
- **Input**: "Improve code generation latency"
- **Expected Output**:
  - Title (string)
  - Problem observed (string)
  - Proposed future (string)
  - Supporting evidence (array)
  - Assumptions (array with confidence levels)
  - Smallest experiment (string)
- **Pass Criteria**:
  - JSON valid and parseable
  - All required fields present
  - Counterarguments array non-empty
  - Status = "proposed"
  - Confidence between 0-1
- **Token Budget**: 2048 output tokens max
- **Automated**: ✅ Parse and validate response

### Test 2.2: Forward-Deployed Engineer (FDE) - Objective Definition
- **Goal**: FDE converts opportunity into business objective
- **Input**: Dreamer output + "Reduce token latency for coding tasks"
- **Expected Output**:
  - Objective statement (string)
  - Outcome hypothesis (string)
  - Success signal (measurable, string)
  - Baseline (optional)
  - Target value (optional)
  - Confidence (0-1)
- **Pass Criteria**:
  - Objective is specific and measurable
  - Outcome hypothesis explains *why* this matters
  - Evidence array includes supporting facts
  - Assumption register entries created
- **Automated**: ✅ Validate structure and logic

### Test 2.3: UX Designer - Contract Generation
- **Goal**: UX Designer produces declarative prototype spec
- **Input**: FDE objective + "Design the UI for latency monitoring"
- **Expected Output**:
  - Screen hierarchy (JSON)
  - Component tree with state management
  - Interactions (local state only)
  - Accessibility requirements
  - Responsive breakpoints
- **Pass Criteria**:
  - Valid JSON schema
  - No arbitrary JavaScript (only declarative)
  - Interactions don't escape prototype boundary
  - Accessibility tags present
- **Automated**: ✅ Schema validation + DeclarativePrototypeRenderer

### Test 2.4: Skeptic / Red Team - Risk & Challenge
- **Goal**: Skeptic identifies risks and disconfirming experiments
- **Input**: FDE objective + UX + proposed approach
- **Expected Output**:
  - Blocking findings (array, high confidence)
  - Non-blocking findings (array)
  - Disconfirmation experiment (string)
  - Recommendation: REJECT | REVISE | EXPERIMENT | PROCEED
- **Pass Criteria**:
  - Finding IDs are unique
  - Recommendation is one of 4 enum values
  - Experiment is specific and falsifiable
  - Reasoning is independent (not just echoing others)
- **Automated**: ✅ Validate structure

### Test 2.5: Architect - Design & ADR
- **Goal**: Architect generates 2+ design options and selects winner
- **Input**: FDE objective + Skeptic findings
- **Expected Output**:
  - Architecture options (array, 2+)
  - Each option includes:
    - Cost estimate (small/medium/large)
    - Security implications (string)
    - Performance impact (string)
    - Maintenance burden (string)
  - Selected ADR with rationale
- **Pass Criteria**:
  - Options are materially different (not subtle variations)
  - ADR status = "approved"
  - Selected option justified by tradeoffs
- **Automated**: ✅ Validate structure and diversity

### Test 2.6: Planner - Task Breakdown
- **Goal**: Planner converts spec into task graph
- **Input**: Approved ADR + UX Contract + requirements
- **Expected Output**:
  - Requirements (array with IDs)
  - Tasks (array with dependencies)
  - Each task includes:
    - Scope (what files/systems affected)
    - Acceptance criteria (array)
    - Max iterations (integer)
    - Allowed paths (RepoGuard scope)
- **Pass Criteria**:
  - Tasks have clear dependencies
  - No circular dependencies
  - All requirements covered by at least one task
  - Task scopes don't escape repo root
- **Automated**: ✅ Graph validation

### Test 2.7: Builder - Implementation
- **Goal**: Builder implements a single task
- **Input**: Task + allowed paths + acceptance criteria
- **Expected Output**:
  - Patch operation (typed, JSON)
  - File modifications in allowed scope
  - Status = "awaiting_operation_approval"
- **Pass Criteria**:
  - Patch is valid (applies cleanly to fixture repo)
  - Files are within approved scope
  - Operation hash matches
  - Evidence captured (built files? tests? metrics?)
- **Automated**: ✅ Via patch-engine validation

### Test 2.8: Verifier - Verification
- **Goal**: Verifier runs tests and validates implementation
- **Input**: Builder patch + acceptance criteria
- **Expected Output**:
  - Verdict: PASS | REVISE | REPLAN | BLOCKED
  - Evidence records (tests run, results)
  - Verdicts for each requirement
- **Pass Criteria**:
  - Verdict supported by evidence
  - Evidence includes timestamps
  - Tests actually ran (not mocked)
- **Automated**: ✅ Validate evidence chain

### Test 2.9: Reviewer - Final Approval
- **Goal**: Reviewer does final sign-off
- **Input**: All prior artifacts + Verifier verdict
- **Expected Output**:
  - Review verdict (string, required)
  - Approval or block (bool)
  - Final notes (optional)
  - Status = "awaiting_merge_review" or "approved"
- **Pass Criteria**:
  - Verdict is independent review
  - All required evidence is in the bundle
  - Approval is explicit (not inferred)
- **Automated**: ✅ Validate verdict

---

## Phase 3: Context Operating System Validation (Tier 2: Important)

### Test 3.1: Token Budget Enforcement
- **Goal**: Context capsule respects token budget and rejects overflow
- **Steps**:
  1. Create large initiative with many requirements/ADRs
  2. Invoke role with small context window (512 tokens)
  3. Verify context OS compiles capsule
  4. If overflow: verify task blocked with `BLOCKED_CONTEXT_OVERFLOW` status
  5. If fit: verify exact token count in capsule
- **Pass Criteria**:
  - Either: Task blocked cleanly OR capsule fits
  - Token count never exceeds available budget
  - Never silent truncation
- **Automated**: ✅ Token boundary tests exist in context-os

### Test 3.2: Role-Specific Context Projection
- **Goal**: Each role gets only its mandatory context
- **Steps**:
  1. Invoke Builder with task context
  2. Verify Builder capsule includes: task, requirements, ADR, spec version
  3. Verify Builder capsule does NOT include: Reviewer notes, Dream contracts
  4. Do same for Verifier (different set of mandatory context)
- **Pass Criteria**:
  - Role gets all mandatory artifacts
  - Role doesn't get unnecessary artifacts
  - Capsule is role-bound (can't be replayed for different role)
- **Automated**: ✅ Projection tests in orchestration-core

### Test 3.3: Context Request Permission Checking
- **Goal**: RepoGuard enforces permissions on context requests
- **Steps**:
  1. Role requests context for file outside allowed paths
  2. Verify request is denied with `RequestDenied` error
  3. Role requests file within scope
  4. Verify request succeeds and new capsule is persisted
- **Pass Criteria**:
  - Permission denied for escaping paths
  - Permission granted for approved paths
  - Request appears in audit log
- **Automated**: ✅ RepoGuard tests exist

### Test 3.4: Stale Binding Detection
- **Goal**: Context OS rejects capsules that reference stale spec/ADR
- **Steps**:
  1. Compile capsule for Spec v1
  2. Change initiative to Spec v2
  3. Try to invoke role with old capsule
  4. Verify rejection with `StaleBinding` error
- **Pass Criteria**:
  - Stale binding detected before inference
  - Error message is clear
  - New capsule can be compiled for v2
- **Automated**: ✅ Stale binding tests in context-os

---

## Phase 4: Dream Mode Autonomy (Tier 2: Important)

### Test 4.1: Dream Mode Requires Mandate
- **Goal**: Dream mode won't start without enabled standing mandate
- **Steps**:
  1. Try to create dream_ideation initiative
  2. Verify blocked without mandate
  3. Create standing mandate (max_iterations=3)
  4. Re-try creation
  5. Verify success
- **Pass Criteria**:
  - Blocked cleanly when no mandate
  - Mandate enforces budget limits
  - Initiative creation succeeds with valid mandate
- **Automated**: ✅ Mandate tests in intent-ledger

### Test 4.2: Dream Mode Budget Enforcement
- **Goal**: Dreamer can't exceed allocated dream budget
- **Steps**:
  1. Create mandate with max_candidates=2
  2. Create dream_ideation initiative
  3. Invoke Dreamer, get 3 opportunities
  4. Try to save 3rd opportunity
  5. Verify 3rd is rejected (budget exceeded)
- **Pass Criteria**:
  - Only 2 dream contracts persisted
  - 3rd rejected with budget error
  - Error is clear and actionable
- **Automated**: ✅ Budget enforcement in orchestration-core

### Test 4.3: Dream Mode No Direct Merge
- **Goal**: Dream work never merges to active branch automatically
- **Steps**:
  1. Create dream_prototype with approved prototype
  2. Build and apply dream worktree changes
  3. Verify changes are in isolated worktree, NOT main branch
  4. Check git status of main (unchanged)
- **Pass Criteria**:
  - Worktree is isolated
  - Main branch unchanged
  - Dream changes only in worktree path
- **Automated**: ✅ Worktree-manager tests

---

## Phase 5: Patch Governance & Transactionality (Tier 1: Critical)

### Test 5.1: Patch Validation & RepoGuard Enforcement
- **Goal**: Patches are validated and confined to approved paths
- **Steps**:
  1. Builder proposes patch modifying src/auth/refresh.ts
  2. Verify patch passes validation
  3. Try patch modifying .git/config
  4. Verify rejected by RepoGuard
- **Pass Criteria**:
  - Valid patches approved
  - Escaping paths rejected
  - Error is clear (path denial)
- **Automated**: ✅ RepoGuard + patch-engine tests

### Test 5.2: Checkpoint & Rollback
- **Goal**: Applied patches can be rolled back atomically
- **Steps**:
  1. Apply patch to fixture repo
  2. Verify files modified
  3. Trigger rollback
  4. Verify original files restored
  5. Check manifest integrity
- **Pass Criteria**:
  - Rollback restores exact original state
  - Manifest remains consistent
  - Audit log records both apply and rollback
- **Automated**: ✅ Checkpoint tests in patch-engine

### Test 5.3: Post-Write DB Failure Recovery
- **Goal**: If DB fails after file write, patch is marked failed & files restored
- **Steps**:
  1. Simulate file write succeeding
  2. Simulate DB write failing during apply finalization
  3. Verify system detects partial failure
  4. Verify automatic restoration from checkpoint
  5. Verify audit log shows failure
- **Pass Criteria**:
  - Files restored automatically
  - Status transitions to apply_failed
  - No silent corruption
- **Automated**: ✅ Failure scenario tests

---

## Phase 6: Pulse Monitoring & Drift Detection (Tier 2: Important)

### Test 6.1: Context Pressure Detection
- **Goal**: Pulse detects when context fills up
- **Steps**:
  1. Create large task graph
  2. Invoke role with small window
  3. Verify context_pressure feature = 1.0
  4. Check pulse event recorded
  5. Verify signal explanation is clear
- **Pass Criteria**:
  - Feature value accurately reflects pressure
  - Event includes timestamp
  - Explanation is actionable
- **Automated**: ✅ Pulse feature tests

### Test 6.2: Assumption Conflict Detection
- **Goal**: Pulse detects when assumption is invalidated
- **Steps**:
  1. Record assumption: "Users prefer quick response"
  2. Run Verifier which finds slow response unacceptable
  3. Verify contradiction detected
  4. Check pulse event created
  5. Verify recommendation to revisit objective
- **Pass Criteria**:
  - Conflict detected
  - Event includes both sources
  - Recommendation is explicit
- **Automated**: ✅ Belief divergence tests in pulse-engine

### Test 6.3: Rework Churn Detection
- **Goal**: Pulse detects repeated revisions (rework churn)
- **Steps**:
  1. Run task through multiple revision cycles (4+)
  2. Verify pulse accumulates evidence of churn
  3. Check rework_churn feature increases
  4. Verify intervention proposed (replan or partition)
- **Pass Criteria**:
  - Churn quantified accurately
  - Feature value increases with iterations
  - Intervention is propositioned (not forced)
- **Automated**: ✅ Churn detection in pulse-engine

---

## Phase 7: Audit & Proof Trail (Tier 1: Critical)

### Test 7.1: Audit Event Completeness
- **Goal**: Every material operation creates audit record
- **Steps**:
  1. Run complete task: Dreamer → Verifier → Reviewer
  2. Query audit log
  3. Verify every major transition is recorded:
     - Initiative created
     - Dreamer invoked
     - Dream contract created
     - ADR approved
     - Task started
     - Patch proposed
     - Patch approved
     - Patch applied
     - Verification passed
     - Review approved
- **Pass Criteria**:
  - All events present
  - Each event has timestamp, actor, payload
  - Payload includes IDs and hashes
- **Automated**: ✅ Via audit_log queries

### Test 7.2: Context Capsule Persistence
- **Goal**: Exact context sent to each role is persisted
- **Steps**:
  1. Invoke role, capture capsule ID
  2. Query persisted capsule
  3. Verify exact_messages array matches sent
  4. Verify context hash is computed
  5. Verify capsule can be reloaded
- **Pass Criteria**:
  - Capsule persisted in DB
  - Messages hash matches calculation
  - Capsule can be restored after restart
- **Automated**: ✅ Context capsule persistence tests

### Test 7.3: Operation Binding Integrity
- **Goal**: Artifacts are immutably bound to their creating operation
- **Steps**:
  1. Builder creates patch
  2. Capture operation hash
  3. Try to apply patch
  4. Verify approval hash matches operation
  5. Change operation hash in DB
  6. Try to apply (should fail)
- **Pass Criteria**:
  - Binding is cryptographic (SHA-256)
  - Tampering detected
  - Apply rejected with binding error
- **Automated**: ✅ Operation binding tests

---

## Phase 8: End-to-End Workflow (Tier 1: Critical)

### Test 8.1: Complete Initiative Lifecycle
**Scenario**: Add simple feature to test repo

**Steps**:
1. Create initiative: "Add refresh token rotation"
2. Invoke Dreamer: Get opportunity assessment
3. Invoke FDE: Define objective & outcome
4. Invoke UX Designer: Create prototype UI
5. Invoke Skeptic: Challenge the approach
6. Invoke Architect: Design implementation
7. Invoke Planner: Break into tasks
8. Invoke Builder: Implement first task
9. Invoke Verifier: Verify implementation
10. Invoke Reviewer: Approve for merge
11. Apply patch to repo
12. Verify main branch is updated
13. Verify worktree is cleaned
14. Query audit log: All events present
15. Export proof report

**Pass Criteria**:
- All 9 roles invoked successfully
- Each produces valid output
- State machine transitions work
- Patch applies cleanly
- Audit trail is complete
- No silent failures

**Success Metrics**:
- Total time: <30 minutes (or document bottlenecks)
- Token efficiency: <80% of window used
- No context overflows
- No rework cycles needed
- All requirements verified as satisfied

**Automated**: ✅ Can script entire flow via Tauri commands

---

## Failure Scenarios (Tier 3: Important)

### Test 9.1: Graceful Context Overflow
- **Expectation**: Task blocked, not silent truncation
- **Verification**: Check BLOCKED_CONTEXT_OVERFLOW status

### Test 9.2: Model Endpoint Failure
- **Expectation**: Error propagated, no retry loop
- **Verification**: Status = "failed", reason = "endpoint_unavailable"

### Test 9.3: Forgery Detection
- **Expectation**: Tampered capsule/approval rejected
- **Verification**: Binding hash check fails before processing

### Test 9.4: Concurrent Initiative Mutations
- **Expectation**: Optimistic locking, last write wins or conflict detected
- **Verification**: Schema handles concurrent updates safely

---

## Validation Checklist

- [ ] Model download complete (20GB)
- [ ] llama.cpp compiled with GPU support
- [ ] llama-server running on localhost:8000
- [ ] IDE starts without errors
- [ ] Runtime capability registered
- [ ] Test 1.1: Server responds
- [ ] Test 1.2: Capability registered
- [ ] Test 1.3: Capsule compiles
- [ ] Test 2.1-2.9: All 9 roles produce valid output
- [ ] Test 3.1-3.4: Context OS enforces budgets
- [ ] Test 4.1-4.3: Dream mode respects mandates
- [ ] Test 5.1-5.3: Patches governed and transactional
- [ ] Test 6.1-6.3: Pulse detects drift
- [ ] Test 7.1-7.3: Audit trail complete
- [ ] Test 8.1: Full workflow succeeds
- [ ] Test 9.1-9.4: Failure modes handled
- [ ] Performance: Task completes in <30 min
- [ ] Zero warnings in build
- [ ] All tests deterministic (rerun yields same results)
- [ ] Production readiness report generated

---

## Success Criteria: PRODUCTION READY

✅ **ALL** of the following must be true:

1. All tests pass (no failures, no ignored tests being skipped)
2. No compilation warnings
3. No TypeScript errors
4. All 9 roles invoked successfully
5. Context budgets enforced (never silent overflow)
6. Patches applied safely (checkpoint/rollback work)
7. Audit trail complete (every operation recorded)
8. Proof report exports without sensitive data (context hidden)
9. End-to-end workflow completes in reasonable time
10. Restart recovery works (database survives process restart)

---

## Performance Targets (Nice to Have)

| Metric | Target | Measurement |
|--------|--------|-------------|
| Dreamer latency | <10s | Time from invocation to response |
| Token efficiency | <75% of window | avg input tokens / window size |
| Patch apply time | <2s | From approval to applied+audit |
| Context compile time | <100ms | Per role, with retrieval |
| Full workflow | <30min | Dreamer through final approval |
| Restart recovery | <500ms | Time to reload session from DB |

---

## Reporting

After all tests complete, generate report including:
- Test execution timestamp
- Model: Qwen 2.5 Coder 32B Q5
- Token budget usage averages
- Any assertion failures (0 expected)
- Audit log sample (show 5 random events)
- Performance metrics
- Pass/fail summary
- Go/no-go decision for production
