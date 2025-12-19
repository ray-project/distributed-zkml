# zkml Component Diagram

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Python Conversion Layer                   │
├─────────────────────────────────────────────────────────────┤
│  converter.py          input_converter.py                   │
│  (TFLite → msgpack)   (numpy → msgpack)                    │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                    Model Representation                      │
├─────────────────────────────────────────────────────────────┤
│  ModelCircuit<F>                                            │
│  ├── DAGLayerConfig (layer sequence)                        │
│  ├── tensors: BTreeMap<i64, Array<F>>                       │
│  ├── used_gadgets: BTreeSet<GadgetType>                      │
│  └── GadgetConfig (circuit configuration)                    │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                    Halo2 Circuit Synthesis                   │
├─────────────────────────────────────────────────────────────┤
│  ModelCircuit::configure()                                  │
│  ├── Configure columns (advice, fixed, instance)             │
│  ├── Configure gadgets (selectors, lookup tables)           │
│  └── Configure commitments (Poseidon)                        │
│                                                              │
│  ModelCircuit::synthesize()                                  │
│  ├── Load lookup tables                                      │
│  ├── Assign constants                                        │
│  ├── Assign input tensors                                    │
│  ├── Commit before (optional)                                 │
│  ├── Execute DAG (forward pass)                              │
│  ├── Commit after (optional)                                 │
│  └── Constrain public values                                 │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                    Proving System                           │
├─────────────────────────────────────────────────────────────┤
│  KZG Backend (BN256)        IPA Backend (Pasta)             │
│  ├── ParamsKZG              ├── ParamsIPA                    │
│  ├── keygen_vk()            ├── keygen_vk()                 │
│  ├── keygen_pk()            ├── keygen_pk()                 │
│  ├── create_proof()        ├── create_proof()              │
│  └── verify_proof()         └── verify_proof()              │
└─────────────────────────────────────────────────────────────┘
```

## Layer Execution Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    DAGLayerChip                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  tensor_map: HashMap<usize, AssignedTensor<F>>              │
│                                                              │
│  for each layer in dag_config.ops:                           │
│    ┌──────────────────────────────────────────┐            │
│    │ 1. Get input tensors by index             │            │
│    │    vec_inps = tensor_map[inp_idxes]       │            │
│    └──────────────────────────────────────────┘            │
│                    │                                         │
│                    ▼                                         │
│    ┌──────────────────────────────────────────┐            │
│    │ 2. Match layer type and call forward()   │            │
│    │    match layer_type:                      │            │
│    │      Conv2D → Conv2DChip::forward()       │            │
│    │      Add → AddChip::forward()              │            │
│    │      FullyConnected → FC::forward()       │            │
│    └──────────────────────────────────────────┘            │
│                    │                                         │
│                    ▼                                         │
│    ┌──────────────────────────────────────────┐            │
│    │ 3. Store outputs in tensor_map             │            │
│    │    tensor_map[out_idx] = output           │            │
│    └──────────────────────────────────────────┘            │
│                                                              │
│  return final_tensor_map[final_out_idxes]                   │
└─────────────────────────────────────────────────────────────┘
```

## Layer → Gadget Mapping

```
┌─────────────────────────────────────────────────────────────┐
│                    Layer Types                               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Conv2D ──────────────┐                                     │
│    │                   │                                     │
│    ├─→ DotProductChip  │                                     │
│    ├─→ AdderChip       │                                     │
│    └─→ ReluChip        │                                     │
│                        │                                     │
│  FullyConnected ───────┤                                     │
│    │                   │                                     │
│    ├─→ DotProductChip  │                                     │
│    ├─→ DotProductBiasChip                                    │
│    └─→ ReluChip        │                                     │
│                        │                                     │
│  Add ──────────────────┤                                     │
│    │                   │                                     │
│    └─→ AddPairsChip    │                                     │
│                        │                                     │
│  Mul ──────────────────┤                                     │
│    │                   │                                     │
│    └─→ MulPairsChip    │                                     │
│                        │                                     │
│  ReLU ─────────────────┤                                     │
│    │                   │                                     │
│    └─→ ReluChip        │                                     │
│                        │                                     │
│  Tanh ─────────────────┤                                     │
│    │                   │                                     │
│    └─→ TanhGadgetChip  │                                     │
│                        │                                     │
│  Sqrt ──────────────────┘                                     │
│    │                                                         │
│    └─→ SqrtGadgetChip                                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Gadget Internal Structure

```
┌─────────────────────────────────────────────────────────────┐
│                    Gadget<F>                                 │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  configure()                                                 │
│  ├── Create selector columns                                │
│  ├── Create lookup table columns                            │
│  └── Define constraints                                      │
│                                                              │
│  load_lookups()                                              │
│  ├── Populate TableColumn with pre-computed values          │
│  └── Store in GadgetConfig                                  │
│                                                              │
│  op_row_region()                                             │
│  ├── Assign input values to advice columns                   │
│  ├── Enable selector                                         │
│  ├── Perform lookup (if needed)                              │
│  └── Assign output to advice column                          │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Commitment Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Commitment System                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  PackerChip                                                  │
│  ├── Pack multiple field elements into fewer                 │
│  ├── Uses bits_per_elem configuration                       │
│  └── Output: packed values                                  │
│                    │                                         │
│                    ▼                                         │
│  PoseidonCommitChip                                          │
│  ├── Hash packed values using Poseidon sponge               │
│  ├── Uses halo2_gadgets::poseidon                           │
│  └── Output: commitment (single field element)               │
│                    │                                         │
│                    ▼                                         │
│  Public Values                                               │
│  ├── Commitment is constrained to instance column           │
│  └── Revealed in proof                                      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Proving Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│                    Complete Proving Flow                     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Setup Phase                                              │
│     ├── Load/generate parameters (KZG/IPA)                  │
│     └── Time: ~1.4s (first run, cached after)               │
│                                                              │
│  2. Key Generation                                           │
│     ├── keygen_vk() - Verifying key                         │
│     │   Time: ~580ms                                        │
│     │   Size: ~25KB                                         │
│     │                                                       │
│     └── keygen_pk() - Proving key                           │
│         Time: ~130ms                                        │
│         Size: ~35MB                                         │
│                                                              │
│  3. Circuit Filling                                          │
│     ├── MockProver::run() - Assign witness values            │
│     ├── Execute synthesis()                                 │
│     └── Time: ~43ms                                         │
│                                                              │
│  4. Proof Generation                                         │
│     ├── create_proof() - Generate ZK-SNARK proof             │
│     ├── Uses transcript (Blake2b)                            │
│     └── Time: ~2.4s                                         │
│                                                              │
│  5. Verification                                             │
│     ├── verify_proof() - Verify proof validity              │
│     └── Time: ~3.9ms                                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## File Organization

```
zkml/
├── src/
│   ├── lib.rs                    # Main library entry
│   ├── model.rs                  # ModelCircuit implementation
│   ├── layers/                   # ML layer implementations
│   │   ├── layer.rs             # Layer trait and types
│   │   ├── dag.rs               # DAG execution
│   │   ├── conv2d.rs            # Convolution layer
│   │   ├── fc/                  # Fully connected layers
│   │   ├── arithmetic/          # Add, Mul, Sub, Div
│   │   └── shape/               # Reshape, Transpose, etc.
│   ├── gadgets/                  # Low-level arithmetic gadgets
│   │   ├── gadget.rs            # Gadget trait
│   │   ├── dot_prod.rs          # Dot product
│   │   ├── adder.rs             # Addition
│   │   └── nonlinear/           # ReLU, tanh, etc.
│   ├── commitments/              # Commitment schemes
│   │   ├── poseidon_commit.rs   # Poseidon hashing
│   │   └── packer.rs            # Bit packing
│   ├── utils/                    # Utilities
│   │   ├── proving_kzg.rs       # KZG proving backend
│   │   ├── proving_ipa.rs       # IPA proving backend
│   │   ├── loader.rs            # Model loading
│   │   └── helpers.rs           # Helper functions
│   └── bin/                      # Binary executables
│       ├── time_circuit.rs      # Benchmark proving
│       ├── test_circuit.rs      # Test circuit correctness
│       └── verify_circuit.rs    # Verify proofs
├── python/                       # Python conversion tools
│   ├── converter.py             # TFLite → msgpack
│   └── input_converter.py       # numpy → msgpack
├── examples/                     # Example models
│   ├── mnist/                   # MNIST example
│   ├── gpt-2/                   # GPT-2 example
│   └── clip/                    # CLIP example
└── halo2/                        # Halo2 fork
    ├── halo2_proofs/            # Proof system
    ├── halo2_gadgets/           # Gadgets library
    └── halo2curves/             # Curve implementations
```

