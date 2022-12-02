pub struct IntegrityChecker;
impl IntegrityChecker {
    /// Checks if the chain is missing the genesis block
    pub fn check_missing_genesis(&self) -> Option<ComponentTypes> {
        if !self.components_received.contains(&ComponentTypes::Genesis) {
            return Some(ComponentTypes::Genesis);
        }

        None
    }

    /// Checks if the chain is missing the Child Block
    pub fn check_missing_child(&self) -> Option<ComponentTypes> {
        if !self.components_received.contains(&ComponentTypes::Child) {
            return Some(ComponentTypes::Child);
        }

        None
    }

    /// Checks if the chain is missing the Parent Block
    pub fn check_missing_parent(&self) -> Option<ComponentTypes> {
        if !self.components_received.contains(&ComponentTypes::Parent) {
            return Some(ComponentTypes::Parent);
        }

        None
    }

    /// Checks if the chain is missing the current ledger
    pub fn check_missing_ledger(&self) -> Option<ComponentTypes> {
        if !self.components_received.contains(&ComponentTypes::Ledger) {
            return Some(ComponentTypes::Ledger);
        }

        None
    }

    /// Checks if the chain is missing the current network state
    pub fn check_missing_state(&self) -> Option<ComponentTypes> {
        if !self
            .components_received
            .contains(&ComponentTypes::NetworkState)
        {
            return Some(ComponentTypes::NetworkState);
        }

        None
    }

    /// Creates vector of all components missing from the chain.
    pub fn check_missing_components(&self) -> Vec<ComponentTypes> {
        let mut missing = vec![];
        if let Some(component) = self.check_missing_genesis() {
            missing.push(component);
        }

        if let Some(component) = self.check_missing_child() {
            missing.push(component);
        }

        if let Some(component) = self.check_missing_parent() {
            missing.push(component);
        }

        if let Some(component) = self.check_missing_ledger() {
            missing.push(component);
        }

        if let Some(component) = self.check_missing_state() {
            missing.push(component);
        }

        missing
    }
}
