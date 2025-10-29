# Design Doc: Profiling, Coverage, and Observability

**Author**: @NiltonVolpato

**Date**: 10-21-2025

## Introduction

### Background

Melbi expressions run in production systems where understanding their
performance characteristics and behavior is critical for operations teams. Users
need to answer questions like:

- "Why is this email filter slow?"
- "Which branch of this feature flag gets executed most often?"
- "Is this ETL pipeline expression actually using all its branches?"
- "Where exactly is the bottleneck in this complex expression?"

Traditional profiling approaches (sampling-based interrupts, manual
instrumentation) have significant drawbacks: they're non-deterministic, add
constant overhead, miss infrequent operations, and provide limited context in VM
environments.

This design proposes an **adaptive profiling system** that automatically
identifies performance bottlenecks with minimal overhead, combined with **code
coverage tracking** and **optional Prometheus integration** for production
observability.

### Current Functionality

None - this is a new feature.

### In Scope

- Adaptive profiling algorithm that automatically drills down into slow
  expressions
- Code coverage tracking for branch/path analysis
- Span mapping infrastructure (shared between profiling, coverage, and error
  reporting)
- Prometheus metrics export (optional, behind feature flag)
- Three-tier instrumentation API (unsafe/dynamic/static)
- Flamegraph-style visualization support
- Production-ready deployment patterns (toggle on/off, sampling, aggregation)

### Out of Scope

- GUI visualization tools (data format only, rendering is external)
- Distributed tracing integration (focus on single-process first)
- Historical trend analysis (just current run data)
- Automatic optimization based on profile data
- Memory profiling (CPU/execution time only for now)
- JIT compilation triggered by profiling

### Assumptions & Dependencies

- Expressions are compiled once and executed many times (amortize
  instrumentation cost)
- Most expressions are fast, and most parts of slow expressions are fast
  (bottlenecks are localized)
- Users care more about production behavior than microbenchmarks
- Span information (source location mapping) is already tracked for error
  reporting
- Atomic operations have acceptable overhead for production counters

### Terminology

- **Adaptive Profiling**: Profiling that recursively subdivides slow parts while
  ignoring fast parts
- **Span**: Source location information (file, line, column range) for a piece
  of code
- **Node**: A point in the AST or bytecode that can be measured (expression,
  operation, function call)
- **Hot Path**: Code path that takes significant execution time
- **Coverage**: Which branches/paths have been executed at least once
- **Hit Count**: How many times a particular branch/path was executed
- **Flamegraph**: Visualization showing hierarchical time attribution
- **Instrumentation**: Adding measurement code to track execution
- **Probe**: A measurement point inserted at a specific node

## Considerations

### Concerns

1. **Performance overhead**: Instrumentation must be lightweight enough for
   production
2. **Memory usage**: Storing counters and timing data for many expressions
3. **Convergence speed**: How many runs until adaptive profiler stabilizes
4. **Granularity tradeoffs**: AST node vs bytecode instruction vs basic block
5. **Thread safety**: Concurrent execution of same expression with profiling
6. **Data export**: Format and mechanism for getting data out
7. **Toggle complexity**: Making it easy to enable/disable without recompilation

### Operational Readiness Considerations

- **Metrics aggregation**: How to roll up data across multiple instances
- **Storage**: Where profile/coverage data lives (in-memory vs persistent)
- **Alerting**: Integration with monitoring systems (Prometheus alerts)
- **Debugging workflow**: How operators use this data to diagnose issues
- **Performance regression detection**: Automatic detection of slowdowns
- **Resource limits**: Prevent profiling from consuming too much memory/CPU

### Open Questions

1. **Granularity**: AST nodes, bytecode instructions, or basic blocks?
2. **Threshold defaults**: What's a reasonable default time threshold (10ms?
   100ms?)?
3. **Convergence**: How many executions before disabling adaptive profiling?
4. **Multi-expression**: Profile single expression vs entire Context?
5. **Async**: How to handle expressions that call async host functions?
6. **Sampling rate**: For very hot paths, should we sample instead of measuring
   every call?
7. **Historical data**: Keep rolling window or just current run?

### Cross-Region Considerations

Not applicable - this is a library.

## Proposed Design

### Solution

A three-part observability system:

1. **Adaptive Profiling**: Automatically identifies bottlenecks by recursively
   measuring slow paths
2. **Code Coverage**: Tracks which branches/paths execute, with optional hit
   counts
3. **Prometheus Integration**: Optional export of metrics for production
   monitoring

All three share a common **span mapping infrastructure** that maps bytecode
positions to source locations, enabling error reporting, coverage, and profiling
to use the same metadata.

The system follows Melbi's three-tier architecture:

- **Unsafe**: No instrumentation, maximum performance
- **Dynamic**: Runtime-toggleable instrumentation with validation
- **Static**: Compile-time profiling configuration with type safety

### System Architecture

```
┌─────────────────────────────────────────────────┐
│           CompiledExpression                    │
│  ┌──────────────┐  ┌────────────────────────┐  │
│  │   Bytecode   │  │  Span Map (shared)     │  │
│  │              │  │  - PC → Source Span    │  │
│  │              │  │  - PC → AST Node       │  │
│  └──────────────┘  └────────────────────────┘  │
└─────────────────────────────────────────────────┘
                      │
        ┌─────────────┼─────────────┐
        │             │             │
┌───────▼────────┐ ┌──▼────────┐ ┌─▼──────────────┐
│   Adaptive     │ │ Coverage  │ │  Prometheus    │
│   Profiler     │ │  Tracker  │ │   Exporter     │
│                │ │           │ │  (optional)    │
│ - Thresholds   │ │ - Hit map │ │  - Histograms  │
│ - Hot nodes    │ │ - Bitsets │ │  - Counters    │
│ - Timing tree  │ │           │ │  - Gauges      │
└────────────────┘ └───────────┘ └────────────────┘
```

### Data Model

#### Span Mapping (Shared Infrastructure)

```rust
pub struct DebugInfo {
    // Maps bytecode PC to source span
    spans: Vec<Span>,
    // Maps bytecode PC to AST node ID
    nodes: Vec<NodeId>,
    // Original source code (for error messages)
    source: String,
}

pub struct Span {
    start: usize,  // Byte offset
    end: usize,
    line: u32,
    column: u32,
}
```

#### Adaptive Profiling

```rust
pub struct AdaptiveProfiler {
    threshold: Duration,
    state: ProfilingState,
    data: ProfileData,
}

enum ProfilingState {
    Disabled,
    Coarse,  // Measuring whole expression
    Adaptive { hot_nodes: HashSet<NodeId> },
}

pub struct ProfileData {
    // Sparse map: only nodes that exceeded threshold
    nodes: HashMap<NodeId, NodeProfile>,
    total_executions: u64,
}

pub struct NodeProfile {
    total_time: Duration,
    hit_count: u64,
    avg_time: Duration,
    children: Option<Vec<(NodeId, NodeProfile)>>,
}
```

#### Code Coverage

```rust
pub struct CoverageTracker {
    // Bitset for executed instructions
    executed: BitVec,
    // Optional: hit counts per instruction
    hit_counts: Option<Vec<AtomicU32>>,
}

pub struct CoverageReport {
    // Per-line coverage
    lines: HashMap<u32, LineCoverage>,
    // Per-branch coverage
    branches: HashMap<NodeId, BranchCoverage>,
    overall_percentage: f64,
}

pub struct LineCoverage {
    hit_count: u64,
    covered: bool,
}

pub struct BranchCoverage {
    branches_taken: Vec<bool>,
    hit_counts: Vec<u64>,
}
```

#### Prometheus Integration

```rust
#[cfg(feature = "prometheus")]
pub struct MetricsCollector {
    execution_duration: Histogram,
    execution_count: Counter,
    error_count: CounterVec,  // labels: error_type

    // Per-expression metrics
    expr_duration: HistogramVec,  // labels: expr_id
    expr_errors: CounterVec,      // labels: expr_id, error_type

    // Coverage metrics
    coverage_percentage: GaugeVec,  // labels: expr_id
    branches_hit: GaugeVec,         // labels: expr_id
}
```

### Interface / API Definitions

#### Adaptive Profiling API

```rust
impl<'ctx, 'arena> CompiledExpression<'ctx, 'arena> {
    // Enable adaptive profiling with threshold
    pub fn with_adaptive_profiling(
        &mut self,
        threshold: Duration,
    ) -> &mut Self;

    // Execute with profiling
    pub fn run_profiled<'val>(
        &mut self,
        profiler: &mut AdaptiveProfiler,
        arena: &'val Bump,
        args: &[Value<'arena, 'val>],
    ) -> Result<Value<'arena, 'val>, RuntimeError>;

    // Get profiling results
    pub fn get_profile(&self) -> Option<&ProfileData>;

    // Export flamegraph data
    pub fn export_flamegraph(&self) -> Result<FlameGraphData, Error>;
}

// Profiler control
impl AdaptiveProfiler {
    pub fn new(threshold: Duration) -> Self;
    pub fn disable(&mut self);
    pub fn reenable(&mut self);
    pub fn reset(&mut self);
    pub fn get_hottest_paths(&self, n: usize) -> Vec<HotPath>;
}
```

#### Coverage API

```rust
impl<'ctx, 'arena> CompiledExpression<'ctx, 'arena> {
    // Enable coverage tracking
    pub fn with_coverage(&mut self, track_hit_counts: bool) -> &mut Self;

    // Execute with coverage
    pub fn run_with_coverage<'val>(
        &mut self,
        coverage: &mut CoverageTracker,
        arena: &'val Bump,
        args: &[Value<'arena, 'val>],
    ) -> Result<Value<'arena, 'val>, RuntimeError>;

    // Get coverage report
    pub fn get_coverage(&self) -> CoverageReport;
}

impl CoverageTracker {
    pub fn new(size: usize, track_counts: bool) -> Self;
    pub fn reset(&mut self);
    pub fn merge(&mut self, other: &CoverageTracker);
}
```

#### Prometheus Integration

```rust
#[cfg(feature = "prometheus")]
impl<'ctx, 'arena> CompiledExpression<'ctx, 'arena> {
    // Wrap with Prometheus metrics
    pub fn with_metrics(
        &mut self,
        collector: Arc<MetricsCollector>,
        expr_id: &str,
    ) -> &mut Self;
}

#[cfg(feature = "prometheus")]
impl MetricsCollector {
    pub fn new(registry: &prometheus::Registry) -> Result<Self, Error>;
    pub fn observe_execution(&self, expr_id: &str, duration: Duration);
    pub fn observe_error(&self, expr_id: &str, error_type: &str);
    pub fn update_coverage(&self, expr_id: &str, percentage: f64);
}
```

### Business Logic

#### Adaptive Profiling Algorithm

```rust
fn execute_adaptive(
    expr: &CompiledExpression,
    profiler: &mut AdaptiveProfiler,
    arena: &Bump,
    args: &[Value],
) -> Result<Value, RuntimeError> {
    match profiler.state {
        ProfilingState::Disabled => {
            // Fast path - zero overhead
            expr.run_unchecked(arena, args)
        }

        ProfilingState::Coarse => {
            // Measure whole expression
            let start = Instant::now();
            let result = expr.run_unchecked(arena, args)?;
            let duration = start.elapsed();

            profiler.data.record_execution(expr.root_node, duration);

            if duration > profiler.threshold {
                // Exceeded threshold - start subdividing
                profiler.state = ProfilingState::Adaptive {
                    hot_nodes: hashset![expr.root_node],
                };
            }

            Ok(result)
        }

        ProfilingState::Adaptive { ref mut hot_nodes } => {
            // Measure only hot nodes and their children
            let result = expr.run_instrumented(arena, args, hot_nodes, profiler)?;

            // Find new hot nodes (children that exceeded threshold)
            for (node_id, profile) in &profiler.data.nodes {
                if profile.avg_time() > profiler.threshold {
                    // Subdivide this node
                    for child in expr.get_children(*node_id) {
                        hot_nodes.insert(child);
                    }
                }
            }

            Ok(result)
        }
    }
}
```

**Key property**: Converges quickly. After a few runs, only the actual
bottlenecks are instrumented.

#### Convergence and Disabling

```rust
impl AdaptiveProfiler {
    pub fn should_disable(&self) -> bool {
        // All hot paths have been subdivided to leaves
        let all_leaf_nodes = self.data.nodes.keys()
            .all(|node| self.expr.is_leaf(*node));

        // Or: ran enough times without finding new hot nodes
        let stable_executions = self.data.total_executions - self.last_subdivision > 100;

        all_leaf_nodes || stable_executions
    }

    pub fn auto_disable_if_stable(&mut self) {
        if self.should_disable() {
            self.disable();
        }
    }
}
```

### Migration Strategy

Not applicable - this is a new feature.

### Work Required

1. **Span Infrastructure** (required for everything)

   - Span tracking during compilation
   - PC → Span mapping
   - PC → NodeId mapping
   - Span → Source text extraction

2. **Adaptive Profiling**

   - ProfilingState machine
   - Instrumentation injection
   - Timing collection (with atomics for thread safety)
   - Hot node identification
   - Convergence detection
   - Flamegraph export format

3. **Code Coverage**

   - BitVec for executed instructions
   - Optional hit count tracking
   - Coverage report generation
   - Branch coverage analysis
   - Line coverage aggregation

4. **Prometheus Integration**

   - Metric definitions
   - Histogram configuration
   - Label management
   - Registry integration
   - Feature flag management

5. **Testing**

   - Unit tests for adaptive algorithm
   - Integration tests with real expressions
   - Performance benchmarks (overhead measurement)
   - Convergence tests
   - Coverage accuracy tests
   - Prometheus export validation

6. **Documentation**
   - User guide for profiling
   - Coverage interpretation guide
   - Prometheus setup instructions
   - Performance tuning recommendations
   - Examples for common scenarios

### Work Sequence

1. Implement span mapping infrastructure (needed by all features)
2. Build basic profiling (non-adaptive, whole-expression timing)
3. Add adaptive subdivision algorithm
4. Implement coverage tracking
5. Add Prometheus integration (behind feature flag)
6. Create visualization export formats (flamegraph, etc.)
7. Performance optimization and overhead reduction
8. Documentation and examples

### High-level Test Plan

- **Unit tests**: Adaptive algorithm logic, convergence, span mapping
- **Integration tests**: Real expressions with known bottlenecks
- **Performance tests**: Measure overhead of instrumentation
- **Accuracy tests**: Verify profiling identifies actual hotspots
- **Coverage tests**: Ensure all branches are tracked correctly
- **Stress tests**: Many expressions, high execution rate
- **Feature flag tests**: Prometheus integration toggles correctly

### Deployment Sequence

Not applicable - this is a library.

## Impact

### Performance

**Without instrumentation (disabled)**: Zero overhead - just branch on
ProfilingState::Disabled

**With coarse profiling**: ~1-5% overhead - single timer around whole expression

**With adaptive profiling (converged)**: <5% overhead - only hot paths
instrumented

**With full coverage**: ~5-10% overhead - every instruction marked as executed

**With Prometheus export**: Minimal additional overhead - metrics updated
asynchronously

### Memory

- **Span map**: ~10-20 bytes per bytecode instruction (already needed for
  errors)
- **Adaptive profiler**: O(hot nodes) - sparse map, typically <100 entries
- **Coverage tracker**: ~1 bit per instruction for basic coverage, 4 bytes per
  instruction for hit counts
- **Prometheus metrics**: Fixed per-metric overhead (~1KB per metric)

### Production Deployment

**Recommended pattern**:

1. Start with adaptive profiling enabled, high threshold (100ms)
2. After warmup (1000 executions), auto-disable if expression is consistently
   fast
3. Periodically re-enable (every 10K executions) to catch regressions
4. Coverage tracking: enable for a sample of executions (1%), aggregate results
5. Prometheus: always enabled if feature compiled in, minimal overhead

### Security

**Coverage and profiling data may leak sensitive information** through
side-channels:

1. **Branch selection reveals conditional outcomes**: If an expression branches
   on `user.role == "admin"`, coverage data shows which branch executed.

2. **Timing differences reveal data characteristics**: Operations on
   different-sized inputs have different timings.

3. **Hit counts reveal data distributions**: Frequency of branch execution can
   reveal statistical properties of sensitive data.

**Recommendations**:

- **Restrict access**: Only authorized operators should access
  profiling/coverage data
- **Aggregate data**: Show aggregate statistics, not per-execution details
- **Sanitize expressions**: Don't profile expressions that branch on PII
- **Document risk**: Make users aware of this limitation
- **Consider noise**: Add timing noise to measurements (reduces precision but
  helps privacy)

#### Trust Model

The ability to write expressions + view observability data = full read access to
all data the expression can access. This is not a bug, it's a fundamental
property.

**Mitigation is organizational, not technical**:

- Separate permissions: expression authors ≠ metrics viewers
- Audit trail: log who wrote which expressions and when
- Review process: require approval before deploying new expressions
- Limit scope: expressions should only access data they need
- Monitor for suspicious patterns: expressions with many similar variants

#### Observability Permissions = Data Permissions

If expression has access to data X, then:

- Observability metrics for that expression are as sensitive as X
- Coverage data is as sensitive as X
- Profiling data is as sensitive as X

User writes expression → Expression runs on Dataset A → Metrics inherit Dataset
A's sensitivity

Observability data has the same sensitivity level as the data the expression
processes:

- **Personal data** → Personal metrics (user-scoped)
- **Company data** → Company metrics (company-scoped)
- **Public data** → Public metrics

**Rule**: Anyone who can view profiling/coverage data can potentially extract
any data the expression can read. Therefore, metrics access must be controlled
with the same rigor as the underlying data.

### Other Aspects

- **Debugging**: Significantly improves ability to diagnose performance issues
- **Operations**: Provides production visibility into expression behavior
- **Development**: Helps developers optimize expressions before deployment
- **Monitoring**: Enables alerting on slow/unused expressions

### Cost Analysis

Not applicable - this is a library.

### Cross-Region Considerations

Not applicable - this is a library.

## Alternatives

### Alternative 1: Sampling-Based Profiling (Traditional)

**Description**: Set periodic interrupt, record current PC when interrupt fires.

**Why discarded**:

- Non-deterministic: can miss infrequent but expensive operations
- Limited context: hard to attribute to logical operations in VM
- Noisy: sample skew, interrupt overhead
- Not suitable for short-running expressions

### Alternative 2: Always-On Full Instrumentation

**Description**: Instrument every node/instruction, always collect data.

**Why discarded**:

- Significant overhead (20-50%)
- Wastes effort on fast parts
- Memory overhead for storing all timing data
- Not production-ready

### Alternative 3: Manual Instrumentation Only

**Description**: Require users to manually add profiling probes.

**Why discarded**:

- Poor UX - users don't know where bottlenecks are
- Doesn't help with expressions they don't control
- Misses optimization opportunities

### Alternative 4: External Profiler Integration (perf, Instruments)

**Description**: Use OS-level profiling tools instead of building custom
solution.

**Why discarded**:

- Doesn't work well for interpreted/VM code
- No access to Melbi's AST/semantic information
- Can't correlate to source spans
- Not portable across platforms

## Looking into the Future

### Next Steps

1. **JIT Compilation Triggers**: Use profiling data to decide what to JIT
   compile
2. **Automatic Optimization**: Suggest or apply optimizations based on profile
3. **Distributed Tracing**: Integrate with OpenTelemetry for cross-service
   tracing
4. **Historical Analysis**: Store and trend profiling data over time
5. **Memory Profiling**: Track allocation patterns and memory hotspots
6. **Comparative Analysis**: Compare profiles across versions to detect
   regressions
7. **GUI Visualization**: Interactive flamegraph explorer

### Nice to Haves

- Real-time profiling dashboard
- Profile-guided optimization recommendations
- Automated A/B testing of expression variations based on profiling
- Integration with CI/CD for performance regression detection
- Machine learning to predict optimal threshold values
- Export to Chrome DevTools trace format
- Profile diffing tool to compare before/after optimization

---

**Document Status**: Initial design **Last Updated**: October 21, 2025
**Maintainers**: @NiltonVolpato
