use plonky2::{
    field::extension::Extendable,
    hash::hash_types::{HashOut, HashOutTarget, RichField},
    iop::{
        target::Target,
        witness::{PartialWitness, Witness},
    },
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::{Proof, ProofWithPublicInputs},
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    poseidon::gadgets::poseidon_two_to_one,
    sparse_merkle_tree::{
        gadgets::process::process_smt::SmtProcessProof, goldilocks_poseidon::WrappedHashOut,
    },
    transaction::gadgets::{
        merge::{MergeProof, MergeTransitionTarget},
        purge::PurgeTransitionTarget,
    },
    zkdsa::account::Address,
};

// type C = PoseidonGoldilocksConfig;
// type H = <C as GenericConfig<D>>::InnerHasher;
// type F = <C as GenericConfig<D>>::F;
// const D: usize = 2;

pub struct MergeAndPurgeTransitionTarget<
    const N_LOG_MAX_USERS: usize,
    const N_LOG_MAX_TXS: usize,
    const N_LOG_MAX_CONTRACTS: usize,
    const N_LOG_MAX_VARIABLES: usize,
    const N_LOG_TXS: usize,
    const N_LOG_RECIPIENTS: usize,
    const N_LOG_CONTRACTS: usize,
    const N_LOG_VARIABLES: usize,
    const N_DIFFS: usize,
    const N_MERGES: usize,
> {
    pub merge_proof_target: MergeTransitionTarget<
        N_LOG_MAX_USERS,
        N_LOG_MAX_TXS,
        N_LOG_TXS,
        N_LOG_RECIPIENTS,
        N_MERGES,
    >,
    pub purge_proof_target: PurgeTransitionTarget<
        N_LOG_MAX_TXS,
        N_LOG_MAX_CONTRACTS,
        N_LOG_MAX_VARIABLES,
        N_LOG_RECIPIENTS,
        N_LOG_CONTRACTS,
        N_LOG_VARIABLES,
        N_DIFFS,
    >,
}

impl<
        const N_LOG_MAX_USERS: usize,
        const N_LOG_MAX_TXS: usize,
        const N_LOG_MAX_CONTRACTS: usize,
        const N_LOG_MAX_VARIABLES: usize,
        const N_LOG_TXS: usize,
        const N_LOG_RECIPIENTS: usize,
        const N_LOG_CONTRACTS: usize,
        const N_LOG_VARIABLES: usize,
        const N_DIFFS: usize,
        const N_MERGES: usize,
    >
    MergeAndPurgeTransitionTarget<
        N_LOG_MAX_USERS,
        N_LOG_MAX_TXS,
        N_LOG_MAX_CONTRACTS,
        N_LOG_MAX_VARIABLES,
        N_LOG_TXS,
        N_LOG_RECIPIENTS,
        N_LOG_CONTRACTS,
        N_LOG_VARIABLES,
        N_DIFFS,
        N_MERGES,
    >
{
    #[allow(clippy::too_many_arguments)]
    pub fn set_witness<F: RichField>(
        &self,
        pw: &mut impl Witness<F>,
        sender_address: Address<F>,
        merge_witnesses: &[MergeProof<F>],
        purge_input_witnesses: &[(SmtProcessProof<F>, SmtProcessProof<F>, SmtProcessProof<F>)],
        purge_output_witnesses: &[(SmtProcessProof<F>, SmtProcessProof<F>, SmtProcessProof<F>)],
        nonce: WrappedHashOut<F>,
        old_user_asset_root: WrappedHashOut<F>,
    ) -> MergeAndPurgeTransitionPublicInputs<F> {
        let middle_user_asset_root =
            self.merge_proof_target
                .set_witness(pw, merge_witnesses, *old_user_asset_root);
        let (new_user_asset_root, diff_root, tx_hash) = self.purge_proof_target.set_witness(
            pw,
            sender_address,
            purge_input_witnesses,
            purge_output_witnesses,
            middle_user_asset_root,
            nonce,
        );

        MergeAndPurgeTransitionPublicInputs {
            sender_address,
            old_user_asset_root,
            middle_user_asset_root,
            new_user_asset_root,
            diff_root,
            tx_hash,
        }
    }
}

pub fn make_user_proof_circuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    const N_LOG_MAX_USERS: usize,
    const N_LOG_MAX_TXS: usize,
    const N_LOG_MAX_CONTRACTS: usize,
    const N_LOG_MAX_VARIABLES: usize,
    const N_LOG_TXS: usize,
    const N_LOG_RECIPIENTS: usize,
    const N_LOG_CONTRACTS: usize,
    const N_LOG_VARIABLES: usize,
    const N_DIFFS: usize,
    const N_MERGES: usize,
>(// zkdsa_circuit: SimpleSignatureCircuit,
) -> MergeAndPurgeTransitionCircuit<
    F,
    C,
    D,
    N_LOG_MAX_USERS,
    N_LOG_MAX_TXS,
    N_LOG_MAX_CONTRACTS,
    N_LOG_MAX_VARIABLES,
    N_LOG_TXS,
    N_LOG_RECIPIENTS,
    N_LOG_CONTRACTS,
    N_LOG_VARIABLES,
    N_DIFFS,
    N_MERGES,
>
where
    C::Hasher: AlgebraicHasher<F>,
{
    // let config = CircuitConfig::standard_recursion_zk_config(); // TODO
    let config = CircuitConfig::standard_recursion_config();

    let mut builder = CircuitBuilder::<F, D>::new(config);
    // builder.debug_gate_row = Some(282);

    let merge_proof_target: MergeTransitionTarget<
        N_LOG_MAX_USERS,
        N_LOG_MAX_TXS,
        N_LOG_TXS,
        N_LOG_RECIPIENTS,
        N_MERGES,
    > = MergeTransitionTarget::add_virtual_to::<F, C::Hasher, D>(&mut builder);

    let purge_proof_target: PurgeTransitionTarget<
        N_LOG_MAX_TXS,
        N_LOG_MAX_CONTRACTS,
        N_LOG_MAX_VARIABLES,
        N_LOG_RECIPIENTS,
        N_LOG_CONTRACTS,
        N_LOG_VARIABLES,
        N_DIFFS,
    > = PurgeTransitionTarget::add_virtual_to::<F, C::Hasher, D>(&mut builder);
    builder.connect_hashes(
        merge_proof_target.new_user_asset_root,
        purge_proof_target.old_user_asset_root,
    );

    let tx_hash = poseidon_two_to_one::<F, C::Hasher, D>(
        &mut builder,
        purge_proof_target.diff_root,
        purge_proof_target.nonce,
    );

    builder.register_public_inputs(&merge_proof_target.old_user_asset_root.elements); // public_inputs[0..4]
    builder.register_public_inputs(&merge_proof_target.new_user_asset_root.elements); // public_inputs[4..8]
    builder.register_public_inputs(&purge_proof_target.new_user_asset_root.elements); // public_inputs[8..12]
    builder.register_public_inputs(&purge_proof_target.diff_root.elements); // public_inputs[12..16]
    builder.register_public_inputs(&purge_proof_target.sender_address.0.elements); // public_inputs[16..20]
    builder.register_public_inputs(&tx_hash.elements); // public_inputs[20..24]

    let targets = MergeAndPurgeTransitionTarget {
        // old_user_asset_root: merge_proof_target.old_user_asset_root,
        // new_user_asset_root: purge_proof_target.new_user_asset_root,
        merge_proof_target,
        purge_proof_target,
        // address: purge_proof_target.sender_address.clone(),
    };

    let merge_and_purge_circuit_data = builder.build::<C>();

    MergeAndPurgeTransitionCircuit {
        data: merge_and_purge_circuit_data,
        targets,
    }
}

pub struct MergeAndPurgeTransitionCircuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    const N_LOG_MAX_USERS: usize,
    const N_LOG_MAX_TXS: usize,
    const N_LOG_MAX_CONTRACTS: usize,
    const N_LOG_MAX_VARIABLES: usize,
    const N_LOG_TXS: usize,
    const N_LOG_RECIPIENTS: usize,
    const N_LOG_CONTRACTS: usize,
    const N_LOG_VARIABLES: usize,
    const N_DIFFS: usize,
    const N_MERGES: usize,
> {
    pub data: CircuitData<F, C, D>,
    pub targets: MergeAndPurgeTransitionTarget<
        N_LOG_MAX_USERS,
        N_LOG_MAX_TXS,
        N_LOG_MAX_CONTRACTS,
        N_LOG_MAX_VARIABLES,
        N_LOG_TXS,
        N_LOG_RECIPIENTS,
        N_LOG_CONTRACTS,
        N_LOG_VARIABLES,
        N_DIFFS,
        N_MERGES,
    >,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "Address<F>: Deserialize<'de>, WrappedHashOut<F>: Deserialize<'de>"))]
pub struct MergeAndPurgeTransitionPublicInputs<F: RichField> {
    pub sender_address: Address<F>,
    pub old_user_asset_root: WrappedHashOut<F>,
    pub middle_user_asset_root: WrappedHashOut<F>,
    pub new_user_asset_root: WrappedHashOut<F>,
    pub diff_root: WrappedHashOut<F>,
    pub tx_hash: WrappedHashOut<F>,
}

impl<F: RichField> MergeAndPurgeTransitionPublicInputs<F> {
    pub fn encode(&self) -> Vec<F> {
        let mut public_inputs = vec![];
        public_inputs.append(&mut self.old_user_asset_root.elements.into());
        public_inputs.append(&mut self.middle_user_asset_root.elements.into());
        public_inputs.append(&mut self.new_user_asset_root.elements.into());
        public_inputs.append(&mut self.diff_root.elements.into());
        public_inputs.append(&mut self.sender_address.elements.into());
        public_inputs.append(&mut self.tx_hash.elements.into());

        public_inputs
    }
}

#[derive(Clone, Debug)]
pub struct MergeAndPurgeTransitionPublicInputsTarget {
    pub sender_address: HashOutTarget,
    pub old_user_asset_root: HashOutTarget,
    pub middle_user_asset_root: HashOutTarget,
    pub new_user_asset_root: HashOutTarget,
    pub diff_root: HashOutTarget,
    pub tx_hash: HashOutTarget,
}

impl MergeAndPurgeTransitionPublicInputsTarget {
    pub fn add_virtual_to<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        let sender_address = builder.add_virtual_hash();
        let old_user_asset_root = builder.add_virtual_hash();
        let middle_user_asset_root = builder.add_virtual_hash();
        let new_user_asset_root = builder.add_virtual_hash();
        let diff_root = builder.add_virtual_hash();
        let tx_hash = builder.add_virtual_hash();

        Self {
            sender_address,
            old_user_asset_root,
            middle_user_asset_root,
            new_user_asset_root,
            diff_root,
            tx_hash,
        }
    }

    pub fn set_witness<F: RichField>(
        &self,
        pw: &mut impl Witness<F>,
        public_inputs: &MergeAndPurgeTransitionPublicInputs<F>,
    ) {
        pw.set_hash_target(self.sender_address, *public_inputs.sender_address);
        pw.set_hash_target(self.old_user_asset_root, *public_inputs.old_user_asset_root);
        pw.set_hash_target(
            self.middle_user_asset_root,
            *public_inputs.middle_user_asset_root,
        );
        pw.set_hash_target(self.new_user_asset_root, *public_inputs.new_user_asset_root);
        pw.set_hash_target(self.diff_root, *public_inputs.diff_root);
        pw.set_hash_target(self.tx_hash, *public_inputs.tx_hash);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct MergeAndPurgeTransitionProofWithPublicInputs<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub proof: Proof<F, C, D>,
    pub public_inputs: MergeAndPurgeTransitionPublicInputs<F>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    From<MergeAndPurgeTransitionProofWithPublicInputs<F, C, D>> for ProofWithPublicInputs<F, C, D>
{
    fn from(
        value: MergeAndPurgeTransitionProofWithPublicInputs<F, C, D>,
    ) -> ProofWithPublicInputs<F, C, D> {
        ProofWithPublicInputs {
            proof: value.proof,
            public_inputs: value.public_inputs.encode(),
        }
    }
}

pub fn parse_merge_and_purge_public_inputs(
    public_inputs_t: &[Target],
) -> MergeAndPurgeTransitionPublicInputsTarget {
    let old_user_asset_root = HashOutTarget {
        elements: public_inputs_t[0..4].try_into().unwrap(),
    };
    let middle_user_asset_root = HashOutTarget {
        elements: public_inputs_t[4..8].try_into().unwrap(),
    };
    let new_user_asset_root = HashOutTarget {
        elements: public_inputs_t[8..12].try_into().unwrap(),
    };
    let diff_root = HashOutTarget {
        elements: public_inputs_t[12..16].try_into().unwrap(),
    };
    let sender_address = HashOutTarget {
        elements: public_inputs_t[16..20].try_into().unwrap(),
    };
    let tx_hash = HashOutTarget {
        elements: public_inputs_t[20..24].try_into().unwrap(),
    };

    MergeAndPurgeTransitionPublicInputsTarget {
        sender_address,
        old_user_asset_root,
        middle_user_asset_root,
        new_user_asset_root,
        diff_root,
        tx_hash,
    }
}

impl<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
        const N_LOG_MAX_USERS: usize,
        const N_LOG_MAX_TXS: usize,
        const N_LOG_MAX_CONTRACTS: usize,
        const N_LOG_MAX_VARIABLES: usize,
        const N_LOG_TXS: usize,
        const N_LOG_RECIPIENTS: usize,
        const N_LOG_CONTRACTS: usize,
        const N_LOG_VARIABLES: usize,
        const N_DIFFS: usize,
        const N_MERGES: usize,
    >
    MergeAndPurgeTransitionCircuit<
        F,
        C,
        D,
        N_LOG_MAX_USERS,
        N_LOG_MAX_TXS,
        N_LOG_MAX_CONTRACTS,
        N_LOG_MAX_VARIABLES,
        N_LOG_TXS,
        N_LOG_RECIPIENTS,
        N_LOG_CONTRACTS,
        N_LOG_VARIABLES,
        N_DIFFS,
        N_MERGES,
    >
{
    pub fn parse_public_inputs(&self) -> MergeAndPurgeTransitionPublicInputsTarget {
        let public_inputs_t = self.data.prover_only.public_inputs.clone();

        parse_merge_and_purge_public_inputs(&public_inputs_t)
    }

    pub fn prove(
        &self,
        inputs: PartialWitness<F>,
    ) -> anyhow::Result<MergeAndPurgeTransitionProofWithPublicInputs<F, C, D>> {
        let proof_with_pis = self.data.prove(inputs)?;
        let public_inputs = proof_with_pis.public_inputs;
        let old_user_asset_root = HashOut {
            elements: public_inputs[0..4].try_into().unwrap(),
        }
        .into();
        let middle_user_asset_root = HashOut {
            elements: public_inputs[4..8].try_into().unwrap(),
        }
        .into();
        let new_user_asset_root = HashOut {
            elements: public_inputs[8..12].try_into().unwrap(),
        }
        .into();
        let diff_root = HashOut {
            elements: public_inputs[12..16].try_into().unwrap(),
        }
        .into();
        let sender_address = Address(HashOut {
            elements: public_inputs[16..20].try_into().unwrap(),
        });
        let tx_hash = HashOut {
            elements: public_inputs[20..24].try_into().unwrap(),
        }
        .into();

        Ok(MergeAndPurgeTransitionProofWithPublicInputs {
            proof: proof_with_pis.proof,
            public_inputs: MergeAndPurgeTransitionPublicInputs {
                sender_address,
                old_user_asset_root,
                middle_user_asset_root,
                new_user_asset_root,
                diff_root,
                tx_hash,
            },
        })
    }

    pub fn verify(
        &self,
        proof_with_pis: MergeAndPurgeTransitionProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<()> {
        let public_inputs = proof_with_pis.public_inputs.encode();

        self.data.verify(ProofWithPublicInputs {
            proof: proof_with_pis.proof,
            public_inputs,
        })
    }
}

/// witness を入力にとり、 user_tx_proof を返す関数
pub fn prove_user_transaction<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    const N_LOG_MAX_USERS: usize,
    const N_LOG_MAX_TXS: usize,
    const N_LOG_MAX_CONTRACTS: usize,
    const N_LOG_MAX_VARIABLES: usize,
    const N_LOG_TXS: usize,
    const N_LOG_RECIPIENTS: usize,
    const N_LOG_CONTRACTS: usize,
    const N_LOG_VARIABLES: usize,
    const N_DIFFS: usize,
    const N_MERGES: usize,
>(
    sender_address: Address<F>,
    merge_witnesses: &[MergeProof<F>],
    purge_input_witnesses: &[(SmtProcessProof<F>, SmtProcessProof<F>, SmtProcessProof<F>)],
    purge_output_witnesses: &[(SmtProcessProof<F>, SmtProcessProof<F>, SmtProcessProof<F>)],
    nonce: WrappedHashOut<F>,
    old_user_asset_root: WrappedHashOut<F>,
) -> anyhow::Result<MergeAndPurgeTransitionProofWithPublicInputs<F, C, D>>
where
    C::Hasher: AlgebraicHasher<F>,
{
    let merge_and_purge_circuit = make_user_proof_circuit::<
        F,
        C,
        D,
        N_LOG_MAX_USERS,
        N_LOG_MAX_TXS,
        N_LOG_MAX_CONTRACTS,
        N_LOG_MAX_VARIABLES,
        N_LOG_TXS,
        N_LOG_RECIPIENTS,
        N_LOG_CONTRACTS,
        N_LOG_VARIABLES,
        N_DIFFS,
        N_MERGES,
    >();

    let mut pw = PartialWitness::new();
    let _public_inputs = merge_and_purge_circuit.targets.set_witness(
        &mut pw,
        sender_address,
        merge_witnesses,
        purge_input_witnesses,
        purge_output_witnesses,
        nonce,
        old_user_asset_root,
    );

    let user_tx_proof = merge_and_purge_circuit
        .prove(pw)
        .map_err(|err| anyhow::anyhow!("fail to prove user transaction: {}", err))?;

    Ok(user_tx_proof)
}
