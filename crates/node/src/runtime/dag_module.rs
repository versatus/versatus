use std::sync::{Arc, RwLock};

use block::{Block, ProposalBlock, ConvergenceBlock, InnerBlock, GenesisBlock};
use bulldag::{graph::{BullDag, GraphError}, vertex::Vertex};
use theater::{ActorState, ActorLabel, ActorId, Handler};
use async_trait::async_trait;
use events::Event;
use telemetry::info;

use crate::EventBroadcastSender;

pub type Edge = (Vertex<Block, String>, Vertex<Block, String>);
pub type Edges = Vec<Edge>;
pub type GraphResult<T> = Result<T, GraphError>;

pub struct DagModule {
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: EventBroadcastSender,
    dag: Arc<RwLock<BullDag<Block, String>>>,
}

impl DagModule {
    pub fn new(
        dag: Arc<RwLock<BullDag<Block, String>>>,
        events_tx: EventBroadcastSender
    ) -> Self {
        Self {
            status: ActorState::Stopped,
            label: String::from("Dag"),
            id: uuid::Uuid::new_v4().to_string(),
            events_tx,
            dag
        }
    }

    pub fn append_genesis(
        &mut self,
        genesis: &GenesisBlock
    ) -> GraphResult<()> {
        let block: Block = genesis.clone().into();
        let vtx: Vertex<Block, String> = block.into();
        self.write_genesis(vtx)?;
        
        Ok(())
    }

    pub fn append_proposal(
        &mut self, 
        proposal: &ProposalBlock) -> GraphResult<()> {
        if let Ok(ref_block) = self.get_reference_block(
            &proposal.ref_block
        ) {
            let block: Block = proposal.clone().into();
            let vtx: Vertex<Block, String> = block.into();
            let edge = (&ref_block, &vtx);
            self.write_edge(edge)?; 
        }; 


        Ok(())
    }

    pub fn append_convergence(
        &mut self,
        convergence: &ConvergenceBlock
    ) -> GraphResult<()> {

        let ref_blocks: Vec<Vertex<Block, String>> = self
            .get_convergence_reference_blocks(
                convergence
        );

        let block: Block = convergence.clone().into();
        let vtx: Vertex<Block, String> = block.into();
        let edges: Edges = ref_blocks.iter().map(|ref_block| {
            (ref_block.clone(), vtx.clone())
        }).collect();

        self.extend_edges(edges)?;

        Ok(())
    }

    fn get_convergence_reference_blocks(
        &self, convergence: &ConvergenceBlock
    ) -> Vec<Vertex<Block, String>> {
        convergence
            .get_ref_hashes()
            .iter()
            .filter_map(|target| {
                match self.get_reference_block(target) {
                    Ok(value) => Some(value),
                    Err(_) => None,
                }
            }).collect()
    }

    fn get_reference_block(
        &self, 
        target: &String
    ) -> GraphResult<Vertex<Block, String>> {

        if let Ok(guard) = self.dag.read() {
            if let Some(vtx) = guard.get_vertex(target.clone()) {
                return Ok(vtx.clone())
            }
        }

        return Err(GraphError::NonExistentReference)
    }

    fn write_edge(
        &mut self, 
        edge: (&Vertex<Block, String>, &Vertex<Block, String>)
    ) -> GraphResult<()> {
        if let Ok(mut guard) = self.dag.write() {
            guard.add_edge(edge);
            return Ok(())
        }

        return Err(GraphError::Other("Error getting write guard".to_string()));
    }

    fn extend_edges(
        &mut self,
        edges: Edges
    ) -> GraphResult<()> {
        let mut iter = edges.iter();
        
        while let Some((ref_block, vtx)) = iter.next() {
            if let Err(e) = self.write_edge((ref_block, vtx)) {
                return Err(e)
            }
        }

        Ok(())
    }

    fn write_genesis(
        &self,
        vertex: &Vertex<Block, String>
    ) -> GraphResult<()> {

        if let Ok(mut guard) = self.dag.write() {
            guard.add_vertex(vertex);

            return Ok(());
        }

        return Err(GraphError::Other("Error getting write gurard".to_string()));
    }
}


#[async_trait]
impl Handler<Event> for DagModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        self.label.clone()
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_start(&self) {
        info!("{}-{} starting", self.label(), self.id(),);
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.label(),
            self.id(),
        );
    }

    async fn handle(&mut self, event: Event) -> theater::Result<ActorState> {
        match event {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            Event::BlockReceived(block) => {
                match block {
                    Block::Genesis { block } => {
                        self.append_genesis(&block);
                    },
                    Block::Proposal { block } => {
                        if let Err(e) = self.append_proposal(&block) {
                            let err_note = format!(
                                "Encountered GraphError: {:?}", e
                            );
                            return Err(theater::TheaterError::Other(err_note));
                        }
                    },
                    Block::Convergence { block } => {
                        if let Err(e) = self.append_convergence(&block) {
                            let err_note = format!(
                                "Encountered GraphError: {:?}", e
                            );
                            return Err(theater::TheaterError::Other(err_note));
                        }
                    }
                }
            },
            Event::NoOp => {},
            // _ => telemetry::warn!("unrecognized command received: {:?}", event),
            _ => {},
        }
        Ok(ActorState::Running)
    }
}
