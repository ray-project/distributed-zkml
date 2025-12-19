# zkml Architecture Deep Dive

## Overview

zkml is a framework for generating zero-knowledge proofs of machine learning model execution using ZK-SNARKs. It converts TFLite models into Halo2 circuits and generates proofs that verify ML inference was computed correctly without revealing the model weights or inputs.

## Core Architecture

### 1. Model Representation (`src/model.rs`)

The `ModelCircuit<F>` is the central data structure that represents an ML model as a ZK-SNARK circuit:

```rust
pub struct ModelCircuit<F: PrimeField> {
  pub used_gadgets: Arc<BTreeSet<GadgetType>>,  // Gadgets needed for this model
  pub dag_config: DAGLayerConfig,                // DAG of layer operations
  pub tensors: BTreeMap<i64, Array<F, IxDyn>>,   // Model weights and inputs
  pub commit_before: Vec<Vec<i64>>,              // Tensors to commit before DAG
  pub commit_after: Vec<Vec<i64>>,               // Tensors to commit after DAG
  pub k: usize,                                  // Circuit degree (2^k rows)
  pub bits_per_elem: usize,                      // Bits per element for packing
  pub inp_idxes: Vec<i64>,                       // Input tensor indices
  pub num_random: i64,                           // Number of random values
}
```

**Key Methods:**
- `generate_from_file()`: Loads model from msgpack files
- `generate_from_msgpack()`: Converts msgpack format to circuit
- `synthesize()`: Implements Halo2 Circuit trait - builds the circuit

### 2. Layer System (`src/layers/`)

Layers represent ML operations (Conv2D, FullyConnected, Pooling, etc.):

**Layer Trait:**
```rust
pub trait Layer<F: PrimeField> {
  fn forward(
    &self,
    layouter: impl Layouter<F>,
    tensors: &Vec<AssignedTensor<F>>,
    constants: &HashMap<i64, CellRc<F>>,
    gadget_config: Rc<GadgetConfig>,
    layer_config: &LayerConfig,
  ) -> Result<Vec<AssignedTensor<F>>, Error>;
}
```

**Supported Layers:**
- **Convolutional**: `Conv2DChip` - 2D convolutions with padding, stride, activation
- **Fully Connected**: `FullyConnectedChip` - Dense/linear layers
- **Pooling**: `AvgPool2DChip`, `MaxPool2DChip`
- **Arithmetic**: `AddChip`, `MulChip`, `SubChip`, `DivVarChip`
- **Activation**: `Relu`, `Tanh`, `Logistic`, `Softmax`
- **Shape Operations**: `Reshape`, `Transpose`, `Slice`, `Pad`, `Broadcast`
- **Special**: `BatchMatMul`, `Cos`, `Sin`, `Pow`, `Sqrt`, `Rsqrt`

### 3. DAG Execution (`src/layers/dag.rs`)

The `DAGLayerChip` executes layers in sequence:

```rust
pub struct DAGLayerConfig {
  pub ops: Vec<LayerConfig>,           // Layer configurations
  pub inp_idxes: Vec<Vec<usize>>,      // Input tensor indices per layer
  pub out_idxes: Vec<Vec<usize>>,      // Output tensor indices per layer
  pub final_out_idxes: Vec<usize>,     // Final output indices
}
```

**Execution Flow:**
1. Maintains a `tensor_map` mapping indices to assigned tensors
2. For each layer, retrieves input tensors by index
3. Calls layer's `forward()` method
4. Stores outputs back in `tensor_map`
5. Returns final output tensors

### 4. Gadget System (`src/gadgets/`)

Gadgets are low-level arithmetic operations optimized for ZK-SNARKs using lookup tables:

**Gadget Trait:**
```rust
pub trait Gadget<F: PrimeField> {
  fn name(&self) -> String;
  fn num_cols_per_op(&self) -> usize;
  fn num_inputs_per_row(&self) -> usize;
  fn num_outputs_per_row(&self) -> usize;
  fn load_lookups(&self, layouter: impl Layouter<F>) -> Result<(), Error>;
  fn op_row_region(&self, region: &mut Region<F>, ...) -> Result<Vec<AssignedCell<F, F>>, Error>;
}
```

**Key Gadgets:**
- **Arithmetic**: `AdderChip`, `DotProductChip`, `MulPairsChip`, `AddPairsChip`
- **Non-linear**: `ReluChip`, `TanhChip`, `ExpGadgetChip`, `SqrtGadgetChip`, `CosGadgetChip`
- **Division**: `VarDivRoundChip`, `VarDivRoundBigChip`, `BiasDivRoundRelu6Chip`
- **Special**: `InputLookupChip` (always used), `UpdateGadgetChip`

**Lookup Tables:**
- Gadgets use Halo2's lookup argument for efficient range checks and non-linear operations
- Pre-computed tables stored in `TableColumn`s
- Lookups are much cheaper than arithmetic constraints for certain operations

### 5. Commitment System (`src/commitments/`)

For privacy-preserving proofs, zkml supports committing to tensors:

**Poseidon Hashing:**
- Uses Poseidon hash function (from `halo2_gadgets`)
- `PoseidonCommitChip` creates commitments to tensor values
- Supports committing before and after DAG execution

**Packing:**
- `PackerChip` packs multiple field elements into fewer elements
- Reduces commitment size by packing bits
- Configurable via `bits_per_elem`

### 6. Proving System (`src/utils/`)

Two proving backends:

**KZG (Kate-Zaverucha-Goldberg):**
- Uses BN256 curve (`halo2curves::bn256::Fr`)
- Faster proving, larger proofs (~3KB for MNIST)
- Trusted setup required (generated on first run)

**IPA (Inner Product Argument):**
- Uses Pasta curves (`halo2curves::pasta::Fp`)
- No trusted setup (transparent)
- Slower proving, smaller proofs

**Proving Flow:**
1. **Setup**: Generate parameters (KZG) or load (IPA)
2. **Key Generation**: 
   - `keygen_vk()` - Generate verifying key
   - `keygen_pk()` - Generate proving key
3. **Circuit Filling**: `MockProver::run()` - Assign values to circuit
4. **Proof Generation**: `create_proof()` - Generate ZK-SNARK proof
5. **Verification**: `verify_proof()` - Verify proof validity

### 7. Model Conversion (`python/`)

Python tools convert TFLite models to zkml format:

**converter.py:**
- Parses TFLite model structure
- Converts operations to zkml layer types
- Quantizes values to fixed-point integers
- Outputs msgpack format with:
  - `global_sf`: Global scale factor
  - `k`: Circuit degree
  - `num_cols`: Number of advice columns
  - `tensors`: Model weights and constants
  - `layers`: Layer configurations

**input_converter.py:**
- Converts numpy arrays to msgpack input format
- Handles quantization and scaling

## Data Flow

### Model Loading
```
TFLite Model → converter.py → model.msgpack (config)
Input Data → input_converter.py → inp.msgpack
```

### Circuit Construction
```
ModelMsgpack → ModelCircuit::generate_from_msgpack()
  ↓
Parse layers → Create DAGLayerConfig
  ↓
Identify used gadgets → Configure GadgetConfig
  ↓
Convert tensors to field elements
```

### Circuit Synthesis (Halo2)
```
ModelCircuit::configure()
  ↓
Configure columns, selectors, lookup tables
  ↓
ModelCircuit::synthesize()
  ↓
1. Load lookup tables for gadgets
2. Assign constants (0, 1, scale_factor, randoms)
3. Assign input tensors
4. Optionally commit to tensors (commit_before)
5. Execute DAG (forward pass through layers)
6. Optionally commit to outputs (commit_after)
7. Constrain public values (commitments + outputs)
```

### Proving
```
Circuit + Witness → create_proof()
  ↓
ZK-SNARK Proof + Public Values
  ↓
verify_proof() → Valid/Invalid
```

## Key Design Decisions

### 1. Fixed-Point Arithmetic
- All values are quantized to fixed-point integers
- Scale factor (`global_sf`) determines precision
- Values stored as `i64`, converted to field elements

### 2. Lookup Tables for Efficiency
- Non-linear operations (ReLU, tanh, etc.) use lookup tables
- Range checks use lookup tables
- Much more efficient than polynomial constraints

### 3. DAG-Based Execution
- Layers executed sequentially in DAG order
- Tensor indices used for data flow
- Supports complex model architectures

### 4. Commitment Support
- Optional commitments for privacy
- Poseidon hashing for commitments
- Packing to reduce commitment size

### 5. Dual Backend Support
- KZG for performance (trusted setup)
- IPA for transparency (no trusted setup)

## Performance Characteristics

### MNIST Example (from our run):
- **Circuit Degree**: k=17 (2^17 = 131,072 rows)
- **Proving Time**: ~2.4 seconds
- **Proof Size**: 3,360 bytes
- **Verification Time**: ~3.9ms
- **Memory**: ~2GB

### Bottlenecks:
1. **Lookup Table Loading**: Pre-computing tables for gadgets
2. **FFT Operations**: Polynomial evaluation in Halo2
3. **MSM (Multi-Scalar Multiplication)**: Commitment operations
4. **Circuit Size**: Larger models = more constraints = slower proving

## Extensibility

### Adding a New Layer:
1. Implement `Layer<F>` trait in `src/layers/`
2. Add to `match_layer()` in `model.rs`
3. Add to DAG execution in `dag.rs`
4. Update converter if needed

### Adding a New Gadget:
1. Implement `Gadget<F>` trait in `src/gadgets/`
2. Add to `GadgetType` enum
3. Configure in `ModelCircuit::configure()`
4. Load lookups in `ModelCircuit::synthesize()`

## Security Considerations

1. **Trusted Setup (KZG)**: Requires secure parameter generation
2. **Lookup Tables**: Must be correctly populated
3. **Public Values**: Outputs and commitments are public
4. **Field Size**: BN256/Pasta fields provide ~128-bit security

## Future Improvements

From code comments and structure:
- Better constant assignment (currently uses advice columns)
- Random oracle for constants (currently deterministic)
- Freivald's algorithm for depthwise convolutions
- Better packing strategies
- Support for more TFLite operations

