use std::{collections::HashMap, marker::PhantomData, rc::Rc};

use halo2_proofs::{circuit::Layouter, halo2curves::ff::{FromUniformBytes, PrimeField}, plonk::Error};

use crate::{
  commitments::merkle::MerkleTreeChip,
  gadgets::gadget::GadgetConfig,
  layers::{
    arithmetic::{add::AddChip, div_var::DivVarChip, mul::MulChip, sub::SubChip},
    batch_mat_mul::BatchMatMulChip,
    cos::CosChip,
    div_fixed::DivFixedChip,
    fc::fully_connected::{FullyConnectedChip, FullyConnectedConfig},
    logistic::LogisticChip,
    max_pool_2d::MaxPool2DChip,
    mean::MeanChip,
    noop::NoopChip,
    pow::PowChip,
    rsqrt::RsqrtChip,
    shape::{
      broadcast::BroadcastChip, concatenation::ConcatenationChip, mask_neg_inf::MaskNegInfChip,
      pack::PackChip, pad::PadChip, permute::PermuteChip, reshape::ReshapeChip,
      resize_nn::ResizeNNChip, rotate::RotateChip, slice::SliceChip, split::SplitChip,
      transpose::TransposeChip,
    },
    sin::SinChip,
    softmax::SoftmaxChip,
    sqrt::SqrtChip,
    square::SquareChip,
    squared_diff::SquaredDiffChip,
    tanh::TanhChip,
    update::UpdateChip,
  },
};

use super::{
  avg_pool_2d::AvgPool2DChip,
  conv2d::Conv2DChip,
  layer::{AssignedTensor, CellRc, GadgetConsumer, Layer, LayerConfig, LayerType},
};

#[derive(Clone, Debug, Default)]
pub struct DAGLayerConfig {
  pub ops: Vec<LayerConfig>,
  pub inp_idxes: Vec<Vec<usize>>,
  pub out_idxes: Vec<Vec<usize>>,
  pub final_out_idxes: Vec<usize>,
}

pub struct DAGLayerChip<F: PrimeField + Ord> {
  dag_config: DAGLayerConfig,
  _marker: PhantomData<F>,
}

impl<F: PrimeField + Ord> DAGLayerChip<F> {
  pub fn construct(dag_config: DAGLayerConfig) -> Self {
    Self {
      dag_config,
      _marker: PhantomData,
    }
  }

  // IMPORTANT: Assumes input tensors are in order. Output tensors can be in any order.
  pub fn forward(
    &self,
    mut layouter: impl Layouter<F>,
    tensors: &Vec<AssignedTensor<F>>,
    constants: &HashMap<i64, CellRc<F>>,
    gadget_config: Rc<GadgetConfig>,
    _layer_config: &LayerConfig,
  ) -> Result<(HashMap<usize, AssignedTensor<F>>, Vec<AssignedTensor<F>>), Error> {
    // Tensor map
    let mut tensor_map = HashMap::new();
    for (idx, tensor) in tensors.iter().enumerate() {
      tensor_map.insert(idx, tensor.clone());
    }

    // Compute the dag
    for (layer_idx, layer_config) in self.dag_config.ops.iter().enumerate() {
      let layer_type = &layer_config.layer_type;
      let inp_idxes = &self.dag_config.inp_idxes[layer_idx];
      let out_idxes = &self.dag_config.out_idxes[layer_idx];
      println!(
        "Processing layer {}, type: {:?}, inp_idxes: {:?}, out_idxes: {:?}, layer_params: {:?}",
        layer_idx, layer_type, inp_idxes, out_idxes, layer_config.layer_params
      );
      let vec_inps = inp_idxes
        .iter()
        .map(|idx| tensor_map.get(&(*idx as usize)).unwrap().clone())
        .collect::<Vec<_>>();

      let out = match layer_type {
        LayerType::Add => {
          let add_chip = AddChip {};
          add_chip.forward(
            layouter.namespace(|| "dag add"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::AvgPool2D => {
          let avg_pool_2d_chip = AvgPool2DChip {};
          avg_pool_2d_chip.forward(
            layouter.namespace(|| "dag avg pool 2d"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::MaxPool2D => {
          let max_pool_2d_chip = MaxPool2DChip {
            marker: PhantomData::<F>,
          };
          max_pool_2d_chip.forward(
            layouter.namespace(|| "dag max pool 2d"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::BatchMatMul => {
          let batch_mat_mul_chip = BatchMatMulChip {};
          batch_mat_mul_chip.forward(
            layouter.namespace(|| "dag batch mat mul"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Broadcast => {
          let broadcast_chip = BroadcastChip {};
          broadcast_chip.forward(
            layouter.namespace(|| "dag batch mat mul"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Conv2D => {
          let conv_2d_chip = Conv2DChip {
            config: layer_config.clone(),
            _marker: PhantomData,
          };
          conv_2d_chip.forward(
            layouter.namespace(|| "dag conv 2d"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Cos => {
          let cos_chip = CosChip {};
          cos_chip.forward(
            layouter.namespace(|| "dag cos"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::DivFixed => {
          let div_fixed_chip = DivFixedChip {};
          div_fixed_chip.forward(
            layouter.namespace(|| "dag div"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::DivVar => {
          let div_var_chip = DivVarChip {};
          div_var_chip.forward(
            layouter.namespace(|| "dag div"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::FullyConnected => {
          let fc_chip = FullyConnectedChip {
            _marker: PhantomData,
            config: FullyConnectedConfig::construct(true),
          };
          fc_chip.forward(
            layouter.namespace(|| "dag fully connected"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Softmax => {
          let softmax_chip = SoftmaxChip {};
          softmax_chip.forward(
            layouter.namespace(|| "dag softmax"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Mean => {
          let mean_chip = MeanChip {};
          mean_chip.forward(
            layouter.namespace(|| "dag mean"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Pad => {
          let pad_chip = PadChip {};
          pad_chip.forward(
            layouter.namespace(|| "dag pad"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Permute => {
          let pad_chip = PermuteChip {};
          pad_chip.forward(
            layouter.namespace(|| "dag permute"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::SquaredDifference => {
          let squared_diff_chip = SquaredDiffChip {};
          squared_diff_chip.forward(
            layouter.namespace(|| "dag squared diff"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Rsqrt => {
          let rsqrt_chip = RsqrtChip {};
          rsqrt_chip.forward(
            layouter.namespace(|| "dag rsqrt"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Sqrt => {
          let sqrt_chip = SqrtChip {};
          sqrt_chip.forward(
            layouter.namespace(|| "dag sqrt"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Logistic => {
          let logistic_chip = LogisticChip {};
          logistic_chip.forward(
            layouter.namespace(|| "dag logistic"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Pow => {
          let pow_chip = PowChip {};
          pow_chip.forward(
            layouter.namespace(|| "dag logistic"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Tanh => {
          let tanh_chip = TanhChip {};
          tanh_chip.forward(
            layouter.namespace(|| "dag tanh"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Mul => {
          let mul_chip = MulChip {};
          mul_chip.forward(
            layouter.namespace(|| "dag mul"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Sin => {
          let sin_chip = SinChip {};
          sin_chip.forward(
            layouter.namespace(|| "dag sin"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Sub => {
          let sub_chip = SubChip {};
          sub_chip.forward(
            layouter.namespace(|| "dag sub"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Noop => {
          let noop_chip = NoopChip {};
          noop_chip.forward(
            layouter.namespace(|| "dag noop"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Transpose => {
          let transpose_chip = TransposeChip {};
          transpose_chip.forward(
            layouter.namespace(|| "dag transpose"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Reshape => {
          let reshape_chip = ReshapeChip {};
          reshape_chip.forward(
            layouter.namespace(|| "dag reshape"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::ResizeNN => {
          let resize_nn_chip = ResizeNNChip {};
          resize_nn_chip.forward(
            layouter.namespace(|| "dag resize nn"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Rotate => {
          let rotate_chip = RotateChip {};
          rotate_chip.forward(
            layouter.namespace(|| "dag rotate"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Concatenation => {
          let concat_chip = ConcatenationChip {};
          concat_chip.forward(
            layouter.namespace(|| "dag concatenation"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Pack => {
          let pack_chip = PackChip {};
          pack_chip.forward(
            layouter.namespace(|| "dag pack"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Split => {
          let split_chip = SplitChip {};
          split_chip.forward(
            layouter.namespace(|| "dag split"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Update => {
          let split_chip = UpdateChip {};
          split_chip.forward(
            layouter.namespace(|| "dag update"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Slice => {
          let slice_chip = SliceChip {};
          slice_chip.forward(
            layouter.namespace(|| "dag slice"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::MaskNegInf => {
          let mask_neg_inf_chip = MaskNegInfChip {};
          mask_neg_inf_chip.forward(
            layouter.namespace(|| "dag mask neg inf"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Square => {
          let square_chip = SquareChip {};
          square_chip.forward(
            layouter.namespace(|| "dag square"),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
      };

      for (idx, tensor_idx) in out_idxes.iter().enumerate() {
        println!("Out {} shape: {:?}", idx, out[idx].shape());
        tensor_map.insert(*tensor_idx as usize, out[idx].clone());
      }
      println!();
    }

    let mut final_out = vec![];
    for idx in self.dag_config.final_out_idxes.iter() {
      final_out.push(tensor_map.get(&(*idx as usize)).unwrap().clone());
    }

    let _print_arr = if final_out.len() > 0 {
      &final_out[0]
    } else {
      if self.dag_config.ops.len() > 0 {
        let last_layer_idx = self.dag_config.ops.len() - 1;
        let out_idx = self.dag_config.out_idxes[last_layer_idx][0];
        tensor_map.get(&out_idx).unwrap()
      } else {
        tensor_map.get(&0).unwrap()
      }
    };

    //let tmp = print_arr.iter().map(|x| x.as_ref()).collect::<Vec<_>>();
    //print_assigned_arr("final out", &tmp.to_vec(), gadget_config.scale_factor);
    //println!("final out idxes: {:?}", self.dag_config.final_out_idxes);

    //let mut x = vec![];
    //for cell in print_arr.iter() {
    //  cell.value().map(|v| {
    //    let bias = 1 << 60 as i64;
    //    let v_pos = *v + F::from(bias as u64);
    //    let v = convert_to_u64(&v_pos) as i64 - bias;
    //    x.push(v);
    //  });
    //}
    //if x.len() > 0 {
    //  let out_fname = "out.msgpack";
    //  let f = File::create(out_fname).unwrap();
    //  let mut buf = BufWriter::new(f);
    //  rmp_serde::encode::write_named(&mut buf, &x).unwrap();
    //}

    Ok((tensor_map, final_out))
  }

  /// Execute only a chunk of layers (for distributed proving)
  /// Returns intermediate values and tensor map after executing layers [start_idx, end_idx)
  pub fn forward_chunk(
    &self,
    mut layouter: impl Layouter<F>,
    tensors: &Vec<AssignedTensor<F>>,
    constants: &HashMap<i64, CellRc<F>>,
    gadget_config: Rc<GadgetConfig>,
    start_idx: usize,
    end_idx: usize,
  ) -> Result<(HashMap<usize, AssignedTensor<F>>, Vec<AssignedTensor<F>>), Error> {
    // Validate indices
    if start_idx >= end_idx || end_idx > self.dag_config.ops.len() {
      return Err(Error::Synthesis);
    }

    // Tensor map
    let mut tensor_map = HashMap::new();
    for (idx, tensor) in tensors.iter().enumerate() {
      tensor_map.insert(idx, tensor.clone());
    }

    // Execute only layers in the chunk range
    for layer_idx in start_idx..end_idx {
      let layer_config = &self.dag_config.ops[layer_idx];
      let layer_type = &layer_config.layer_type;
      let inp_idxes = &self.dag_config.inp_idxes[layer_idx];
      let out_idxes = &self.dag_config.out_idxes[layer_idx];
      
      println!(
        "Processing chunk layer {}, type: {:?}, inp_idxes: {:?}, out_idxes: {:?}",
        layer_idx, layer_type, inp_idxes, out_idxes
      );
      
      let vec_inps = inp_idxes
        .iter()
        .map(|idx| tensor_map.get(&(*idx as usize)).unwrap().clone())
        .collect::<Vec<_>>();

      // Execute the layer (same logic as forward, but only for chunk range)
      let out = match layer_type {
        LayerType::Add => {
          let add_chip = AddChip {};
          add_chip.forward(
            layouter.namespace(|| format!("chunk add {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::AvgPool2D => {
          let avg_pool_2d_chip = AvgPool2DChip {};
          avg_pool_2d_chip.forward(
            layouter.namespace(|| format!("chunk avg pool 2d {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::MaxPool2D => {
          let max_pool_2d_chip = MaxPool2DChip {
            marker: PhantomData::<F>,
          };
          max_pool_2d_chip.forward(
            layouter.namespace(|| format!("chunk max pool 2d {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::BatchMatMul => {
          let batch_mat_mul_chip = BatchMatMulChip {};
          batch_mat_mul_chip.forward(
            layouter.namespace(|| format!("chunk batch mat mul {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Broadcast => {
          let broadcast_chip = BroadcastChip {};
          broadcast_chip.forward(
            layouter.namespace(|| format!("chunk broadcast {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Conv2D => {
          let conv_2d_chip = Conv2DChip {
            config: layer_config.clone(),
            _marker: PhantomData,
          };
          conv_2d_chip.forward(
            layouter.namespace(|| format!("chunk conv 2d {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Cos => {
          let cos_chip = CosChip {};
          cos_chip.forward(
            layouter.namespace(|| format!("chunk cos {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::DivFixed => {
          let div_fixed_chip = DivFixedChip {};
          div_fixed_chip.forward(
            layouter.namespace(|| format!("chunk div fixed {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::DivVar => {
          let div_var_chip = DivVarChip {};
          div_var_chip.forward(
            layouter.namespace(|| format!("chunk div var {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::FullyConnected => {
          let fc_chip = FullyConnectedChip {
            _marker: PhantomData,
            config: FullyConnectedConfig::construct(true),
          };
          fc_chip.forward(
            layouter.namespace(|| format!("chunk fully connected {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Softmax => {
          let softmax_chip = SoftmaxChip {};
          softmax_chip.forward(
            layouter.namespace(|| format!("chunk softmax {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Mean => {
          let mean_chip = MeanChip {};
          mean_chip.forward(
            layouter.namespace(|| format!("chunk mean {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Pad => {
          let pad_chip = PadChip {};
          pad_chip.forward(
            layouter.namespace(|| format!("chunk pad {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Permute => {
          let permute_chip = PermuteChip {};
          permute_chip.forward(
            layouter.namespace(|| format!("chunk permute {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::SquaredDifference => {
          let squared_diff_chip = SquaredDiffChip {};
          squared_diff_chip.forward(
            layouter.namespace(|| format!("chunk squared diff {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Rsqrt => {
          let rsqrt_chip = RsqrtChip {};
          rsqrt_chip.forward(
            layouter.namespace(|| format!("chunk rsqrt {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Sqrt => {
          let sqrt_chip = SqrtChip {};
          sqrt_chip.forward(
            layouter.namespace(|| format!("chunk sqrt {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Logistic => {
          let logistic_chip = LogisticChip {};
          logistic_chip.forward(
            layouter.namespace(|| format!("chunk logistic {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Pow => {
          let pow_chip = PowChip {};
          pow_chip.forward(
            layouter.namespace(|| format!("chunk pow {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Tanh => {
          let tanh_chip = TanhChip {};
          tanh_chip.forward(
            layouter.namespace(|| format!("chunk tanh {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Mul => {
          let mul_chip = MulChip {};
          mul_chip.forward(
            layouter.namespace(|| format!("chunk mul {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Sin => {
          let sin_chip = SinChip {};
          sin_chip.forward(
            layouter.namespace(|| format!("chunk sin {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Sub => {
          let sub_chip = SubChip {};
          sub_chip.forward(
            layouter.namespace(|| format!("chunk sub {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Noop => {
          let noop_chip = NoopChip {};
          noop_chip.forward(
            layouter.namespace(|| format!("chunk noop {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Transpose => {
          let transpose_chip = TransposeChip {};
          transpose_chip.forward(
            layouter.namespace(|| format!("chunk transpose {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Reshape => {
          let reshape_chip = ReshapeChip {};
          reshape_chip.forward(
            layouter.namespace(|| format!("chunk reshape {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::ResizeNN => {
          let resize_nn_chip = ResizeNNChip {};
          resize_nn_chip.forward(
            layouter.namespace(|| format!("chunk resize nn {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Rotate => {
          let rotate_chip = RotateChip {};
          rotate_chip.forward(
            layouter.namespace(|| format!("chunk rotate {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Concatenation => {
          let concat_chip = ConcatenationChip {};
          concat_chip.forward(
            layouter.namespace(|| format!("chunk concatenation {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Pack => {
          let pack_chip = PackChip {};
          pack_chip.forward(
            layouter.namespace(|| format!("chunk pack {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Split => {
          let split_chip = SplitChip {};
          split_chip.forward(
            layouter.namespace(|| format!("chunk split {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Update => {
          let update_chip = UpdateChip {};
          update_chip.forward(
            layouter.namespace(|| format!("chunk update {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Slice => {
          let slice_chip = SliceChip {};
          slice_chip.forward(
            layouter.namespace(|| format!("chunk slice {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::MaskNegInf => {
          let mask_neg_inf_chip = MaskNegInfChip {};
          mask_neg_inf_chip.forward(
            layouter.namespace(|| format!("chunk mask neg inf {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
        LayerType::Square => {
          let square_chip = SquareChip {};
          square_chip.forward(
            layouter.namespace(|| format!("chunk square {}", layer_idx)),
            &vec_inps,
            constants,
            gadget_config.clone(),
            &layer_config,
          )?
        }
      };

      // Store outputs in tensor map
      for (idx, tensor_idx) in out_idxes.iter().enumerate() {
        tensor_map.insert(*tensor_idx as usize, out[idx].clone());
      }
    }

    // Extract intermediate values (outputs of last layer in chunk)
    // Return as tensors (will be flattened to cells when building Merkle tree)
    let mut intermediate_tensors = Vec::new();
    if end_idx > 0 {
      let last_layer_idx = end_idx - 1;
      let out_idxes = &self.dag_config.out_idxes[last_layer_idx];
      for tensor_idx in out_idxes.iter() {
        if let Some(tensor) = tensor_map.get(&(*tensor_idx as usize)) {
          intermediate_tensors.push(tensor.clone());
        }
      }
    }

    Ok((tensor_map, intermediate_tensors))
  }

  /// Execute a chunk and build Merkle tree from intermediate values
  /// Returns tensor map, intermediate tensors, and Merkle root
  pub fn forward_chunk_with_merkle(
    &self,
    mut layouter: impl Layouter<F>,
    tensors: &Vec<AssignedTensor<F>>,
    constants: &HashMap<i64, CellRc<F>>,
    gadget_config: Rc<GadgetConfig>,
    start_idx: usize,
    end_idx: usize,
    merkle_chip: &MerkleTreeChip<F>,
  ) -> Result<(HashMap<usize, AssignedTensor<F>>, Vec<AssignedTensor<F>>, CellRc<F>), Error>
  where
    F: PrimeField + Ord + FromUniformBytes<64>,
  {
    // Execute chunk
    let (tensor_map, intermediate_tensors) = self.forward_chunk(
      layouter.namespace(|| "chunk execution"),
      tensors,
      constants,
      gadget_config.clone(),
      start_idx,
      end_idx,
    )?;

    // Flatten intermediate tensors to individual cells for Merkle tree
    let mut intermediate_cells = Vec::new();
    for tensor in &intermediate_tensors {
      for cell in tensor.iter() {
        intermediate_cells.push(cell.clone());
      }
    }

    // Build Merkle tree from intermediate values
    let merkle_root = if intermediate_cells.is_empty() {
      // If no intermediate values, return zero hash
      let zero = constants.get(&0).ok_or(Error::Synthesis)?;
      merkle_chip.hash_single(
        layouter.namespace(|| "empty merkle"),
        gadget_config,
        constants,
        zero.clone(),
      )?
    } else {
      merkle_chip.build_binary_tree(
        layouter.namespace(|| "merkle tree"),
        gadget_config,
        constants,
        &intermediate_cells,
      )?
    };

    Ok((tensor_map, intermediate_tensors, merkle_root))
  }
}

impl<F: PrimeField + Ord> GadgetConsumer for DAGLayerChip<F> {
  // Special case: DAG doesn't do anything
  fn used_gadgets(&self, _layer_config: &LayerConfig) -> Vec<crate::gadgets::gadget::GadgetType> {
    vec![]
  }
}
