use std::{
  fs::File,
  io::{BufReader, Write},
  path::Path,
  time::Instant,
};

use halo2_proofs::{
  dev::MockProver,
  halo2curves::bn256::{Bn256, Fr, G1Affine},
  plonk::{create_proof, keygen_pk, keygen_vk, verify_proof, VerifyingKey},
  poly::{
    commitment::Params,
    kzg::{
      commitment::{KZGCommitmentScheme, ParamsKZG},
      multiopen::{ProverSHPLONK, VerifierSHPLONK},
      strategy::SingleStrategy,
    },
  },
  transcript::{
    Blake2bRead, Blake2bWrite, Challenge255, TranscriptReadBuffer, TranscriptWriterBuffer,
  },
  SerdeFormat,
};

use crate::{model::ModelCircuit, utils::helpers::get_public_values};

pub fn get_kzg_params(params_dir: &str, degree: u32) -> ParamsKZG<Bn256> {
  let rng = rand::thread_rng();
  let path = format!("{}/{}.params", params_dir, degree);
  let params_path = Path::new(&path);
  if File::open(&params_path).is_err() {
    let params = ParamsKZG::<Bn256>::setup(degree, rng);
    let mut buf = Vec::new();

    params.write(&mut buf).expect("Failed to write params");
    let mut file = File::create(&params_path).expect("Failed to create params file");
    file
      .write_all(&buf[..])
      .expect("Failed to write params to file");
  }

  let mut params_fs = File::open(&params_path).expect("couldn't load params");
  let params = ParamsKZG::<Bn256>::read(&mut params_fs).expect("Failed to read params");
  params
}

pub fn serialize(data: &Vec<u8>, path: &str) -> u64 {
  let mut file = File::create(path).unwrap();
  file.write_all(data).unwrap();
  file.metadata().unwrap().len()
}

pub fn verify_kzg(
  params: &ParamsKZG<Bn256>,
  vk: &VerifyingKey<G1Affine>,
  strategy: SingleStrategy<Bn256>,
  public_vals: &Vec<Fr>,
  mut transcript: Blake2bRead<&[u8], G1Affine, Challenge255<G1Affine>>,
) {
  assert!(
    verify_proof::<
      KZGCommitmentScheme<Bn256>,
      VerifierSHPLONK<'_, Bn256>,
      Challenge255<G1Affine>,
      Blake2bRead<&[u8], G1Affine, Challenge255<G1Affine>>,
      halo2_proofs::poly::kzg::strategy::SingleStrategy<'_, Bn256>,
    >(&params, &vk, strategy, &[&[&public_vals]], &mut transcript)
    .is_ok(),
    "proof did not verify"
  );
}

pub fn time_circuit_kzg(circuit: ModelCircuit<Fr>) {
  let rng = rand::thread_rng();
  let start = Instant::now();

  let degree = circuit.k as u32;
  let params = get_kzg_params("./params_kzg", degree);

  let circuit_duration = start.elapsed();
  println!(
    "Time elapsed in params construction: {:?}",
    circuit_duration
  );

  let vk_circuit = circuit.clone();
  let vk = keygen_vk(&params, &vk_circuit).unwrap();
  drop(vk_circuit);
  let vk_duration = start.elapsed();
  println!(
    "Time elapsed in generating vkey: {:?}",
    vk_duration - circuit_duration
  );

  let vkey_size = serialize(&vk.to_bytes(SerdeFormat::RawBytes), "vkey");
  println!("vkey size: {} bytes", vkey_size);

  let pk_circuit = circuit.clone();
  let pk = keygen_pk(&params, vk, &pk_circuit).unwrap();
  let pk_duration = start.elapsed();
  println!(
    "Time elapsed in generating pkey: {:?}",
    pk_duration - vk_duration
  );
  drop(pk_circuit);

  let pkey_size = serialize(&pk.to_bytes(SerdeFormat::RawBytes), "pkey");
  println!("pkey size: {} bytes", pkey_size);

  let fill_duration = start.elapsed();
  let proof_circuit = circuit.clone();
  let _prover = MockProver::run(degree, &proof_circuit, vec![vec![]]).unwrap();
  let public_vals = get_public_values();
  println!(
    "Time elapsed in filling circuit: {:?}",
    fill_duration - pk_duration
  );

  // Convert public vals to serializable format
  let public_vals_u8: Vec<u8> = public_vals
    .iter()
    .map(|v: &Fr| v.to_bytes().to_vec())
    .flatten()
    .collect();
  let public_vals_u8_size = serialize(&public_vals_u8, "public_vals");
  println!("Public vals size: {} bytes", public_vals_u8_size);

  let proof_duration_start = start.elapsed();
  let mut transcript = Blake2bWrite::<_, G1Affine, Challenge255<_>>::init(vec![]);
  create_proof::<
    KZGCommitmentScheme<Bn256>,
    ProverSHPLONK<'_, Bn256>,
    Challenge255<G1Affine>,
    _,
    Blake2bWrite<Vec<u8>, G1Affine, Challenge255<G1Affine>>,
    ModelCircuit<Fr>,
  >(
    &params,
    &pk,
    &[proof_circuit],
    &[&[&public_vals]],
    rng,
    &mut transcript,
  )
  .unwrap();
  let proof = transcript.finalize();
  let proof_duration = start.elapsed();
  println!("Proving time: {:?}", proof_duration - proof_duration_start);

  let proof_size = serialize(&proof, "proof");
  let proof = std::fs::read("proof").unwrap();

  println!("Proof size: {} bytes", proof_size);

  let strategy = SingleStrategy::new(&params);
  let transcript_read = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);

  println!("public vals: {:?}", public_vals);
  verify_kzg(
    &params,
    &pk.get_vk(),
    strategy,
    &public_vals,
    transcript_read,
  );
  let verify_duration = start.elapsed();
  println!("Verifying time: {:?}", verify_duration - proof_duration);
}

/// Result of proving a chunk
pub struct ChunkProofResult {
  /// The serialized proof bytes
  pub proof: Vec<u8>,
  /// Public values including Merkle root (if use_merkle=true)
  pub public_vals: Vec<Fr>,
  /// The Merkle root (last public value if use_merkle=true)
  pub merkle_root: Option<Fr>,
  /// Proving time in milliseconds
  pub proving_time_ms: u128,
  /// Verification time in milliseconds
  pub verify_time_ms: u128,
}

/// Generate a KZG proof for a chunk of the model
/// 
/// # Arguments
/// * `config_path` - Path to model config msgpack file
/// * `input_path` - Path to input msgpack file
/// * `chunk_start` - Starting layer index (inclusive)
/// * `chunk_end` - Ending layer index (exclusive)
/// * `use_merkle` - Whether to compute and include Merkle root in public values
/// * `prev_merkle_root` - Optional Merkle root from previous chunk (for chained verification)
/// * `params_dir` - Directory for KZG params (will be created if needed)
/// 
/// # Returns
/// ChunkProofResult containing proof, public values, and timing info
pub fn prove_chunk_kzg(
  config_path: &str,
  input_path: &str,
  chunk_start: usize,
  chunk_end: usize,
  use_merkle: bool,
  prev_merkle_root: Option<Fr>,
  params_dir: &str,
) -> ChunkProofResult {
  use crate::utils::loader::load_model_msgpack;
  
  let _start = Instant::now();
  
  // Load and configure circuit for chunk execution
  let config = load_model_msgpack(config_path, input_path);
  let mut circuit = ModelCircuit::<Fr>::generate_from_file(config_path, input_path);
  
  // If using Merkle, ensure Poseidon hasher is configured
  if use_merkle && circuit.commit_after.is_empty() && circuit.commit_before.is_empty() {
    // Set commit_after to enable Poseidon hasher
    if let Some(layer) = config.layers.get(chunk_end.saturating_sub(1)) {
      if !layer.out_idxes.is_empty() {
        circuit.commit_after = vec![layer.out_idxes.clone()];
      }
    }
  }
  
  // Configure for chunk execution
  circuit.set_chunk_config(chunk_start, chunk_end, use_merkle);
  
  // Set previous Merkle root for chained verification (if provided)
  if let Some(prev_root) = prev_merkle_root {
    circuit.set_prev_merkle_root(prev_root);
  }
  
  let degree = circuit.k as u32;
  let params = get_kzg_params(params_dir, degree);
  
  // Generate keys
  let vk = keygen_vk(&params, &circuit).unwrap();
  let pk = keygen_pk(&params, vk, &circuit).unwrap();
  
  // First run to get public values
  let _mock = MockProver::run(degree, &circuit, vec![vec![]]).unwrap();
  let public_vals = get_public_values();
  
  // Extract Merkle root (last public value if use_merkle)
  let merkle_root = if use_merkle && !public_vals.is_empty() {
    Some(public_vals[public_vals.len() - 1])
  } else {
    None
  };
  
  // Generate proof
  let rng = rand::thread_rng();
  let prove_start = Instant::now();
  let mut transcript = Blake2bWrite::<_, G1Affine, Challenge255<_>>::init(vec![]);
  create_proof::<
    KZGCommitmentScheme<Bn256>,
    ProverSHPLONK<'_, Bn256>,
    Challenge255<G1Affine>,
    _,
    Blake2bWrite<Vec<u8>, G1Affine, Challenge255<G1Affine>>,
    ModelCircuit<Fr>,
  >(
    &params,
    &pk,
    &[circuit],
    &[&[&public_vals]],
    rng,
    &mut transcript,
  )
  .unwrap();
  let proof = transcript.finalize();
  let proving_time_ms = prove_start.elapsed().as_millis();
  
  // Verify the proof
  let verify_start = Instant::now();
  let strategy = SingleStrategy::new(&params);
  let transcript_read = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
  verify_kzg(&params, pk.get_vk(), strategy, &public_vals, transcript_read);
  let verify_time_ms = verify_start.elapsed().as_millis();
  
  println!(
    "Chunk [{}, {}): proof generated in {}ms, verified in {}ms, {} public vals{}",
    chunk_start,
    chunk_end,
    proving_time_ms,
    verify_time_ms,
    public_vals.len(),
    if use_merkle { ", includes Merkle root" } else { "" }
  );
  
  ChunkProofResult {
    proof,
    public_vals,
    merkle_root,
    proving_time_ms,
    verify_time_ms,
  }
}

// Standalone verification
pub fn verify_circuit_kzg(
  circuit: ModelCircuit<Fr>,
  vkey_fname: &str,
  proof_fname: &str,
  public_vals_fname: &str,
) {
  let degree = circuit.k as u32;
  let params = get_kzg_params("./params_kzg", degree);
  println!("Loaded the parameters");

  let vk = VerifyingKey::read::<BufReader<File>, ModelCircuit<Fr>>(
    &mut BufReader::new(File::open(vkey_fname).unwrap()),
    SerdeFormat::RawBytes,
    (),
  )
  .unwrap();
  println!("Loaded vkey");

  let proof = std::fs::read(proof_fname).unwrap();

  let public_vals_u8 = std::fs::read(&public_vals_fname).unwrap();
  let public_vals: Vec<Fr> = public_vals_u8
    .chunks(32)
    .map(|chunk| Fr::from_bytes(chunk.try_into().expect("conversion failed")).unwrap())
    .collect();

  let strategy = SingleStrategy::new(&params);
  let transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);

  let start = Instant::now();
  let verify_start = start.elapsed();
  verify_kzg(&params, &vk, strategy, &public_vals, transcript);
  let verify_duration = start.elapsed();
  println!("Verifying time: {:?}", verify_duration - verify_start);
  println!("Proof verified!")
}
