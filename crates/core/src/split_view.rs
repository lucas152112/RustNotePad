use std::collections::HashMap;
use std::path::PathBuf;

/// 視窗分割面板。 / Identifies which pane a tab belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Primary,
    Secondary,
}

/// 多重執行個體策略。 / Strategy for handling multi-instance behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiInstancePolicy {
    SingleWindow,
    CloneOnDemand,
    AlwaysNewInstance,
}

/// 標籤識別碼。 / Opaque identifier for a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(usize);

impl TabId {
    fn next(value: usize) -> Self {
        TabId(value)
    }
}

/// 單一標籤的狀態。 / Metadata tracked per open tab.
#[derive(Debug, Clone)]
pub struct TabRecord {
    pub id: TabId,
    pub title: String,
    pub path: Option<PathBuf>,
    pub is_dirty: bool,
}

#[derive(Debug, Default, Clone)]
struct PaneState {
    tabs: Vec<TabId>,
    active: Option<TabId>,
}

impl PaneState {
    fn activate(&mut self, id: TabId) {
        if !self.tabs.contains(&id) {
            self.tabs.push(id);
        }
        self.active = Some(id);
    }

    fn remove(&mut self, id: TabId) {
        self.tabs.retain(|&tab| tab != id);
        if self.active == Some(id) {
            self.active = self.tabs.last().copied();
        }
    }
}

/// 管理分割視窗與標籤狀態。 / Tracks tabs across split views and multi-instance strategy.
#[derive(Debug, Clone)]
pub struct SplitViewState {
    panes: [PaneState; 2],
    tabs: HashMap<TabId, TabRecord>,
    next_tab: usize,
    policy: MultiInstancePolicy,
}

impl Default for SplitViewState {
    fn default() -> Self {
        Self::new(MultiInstancePolicy::SingleWindow)
    }
}

impl SplitViewState {
    /// 以指定策略建立管理器。 / Creates a manager with the given policy.
    pub fn new(policy: MultiInstancePolicy) -> Self {
        Self {
            panes: [PaneState::default(), PaneState::default()],
            tabs: HashMap::new(),
            next_tab: 0,
            policy,
        }
    }

    /// 目前策略。 / Returns the current multi-instance policy.
    pub fn policy(&self) -> MultiInstancePolicy {
        self.policy
    }

    /// 更新策略。 / Adjusts the policy.
    pub fn set_policy(&mut self, policy: MultiInstancePolicy) {
        self.policy = policy;
    }

    /// 在指定面板開啟新標籤。 / Opens a new tab in the target pane.
    pub fn open_tab(
        &mut self,
        pane: Pane,
        title: impl Into<String>,
        path: Option<PathBuf>,
    ) -> TabId {
        let id = TabId::next(self.next_tab);
        self.next_tab += 1;
        let record = TabRecord {
            id,
            title: title.into(),
            path,
            is_dirty: false,
        };
        self.tabs.insert(id, record);
        self.pane_mut(pane).activate(id);
        id
    }

    /// 將標籤移動至另一面板。 / Moves a tab across panes.
    pub fn move_to(&mut self, id: TabId, target: Pane) -> bool {
        if !self.tabs.contains_key(&id) {
            return false;
        }
        let other = self.other_pane(target);
        self.pane_mut(other).remove(id);
        self.pane_mut(target).activate(id);
        true
    }

    /// 複製標籤到另一面板。 / Clones a tab into the opposite pane.
    pub fn clone_to_other(&mut self, id: TabId) -> Option<TabId> {
        let record = self.tabs.get(&id)?.clone();
        let new_id = TabId::next(self.next_tab);
        self.next_tab += 1;
        let mut clone = record.clone();
        clone.id = new_id;
        self.tabs.insert(new_id, clone);
        let target = self.other_pane(self.locate_pane(id)?);
        self.pane_mut(target).activate(new_id);
        Some(new_id)
    }

    /// 關閉標籤。 / Closes the referenced tab.
    pub fn close_tab(&mut self, id: TabId) -> bool {
        let pane = match self.locate_pane(id) {
            Some(pane) => pane,
            None => return false,
        };
        self.pane_mut(pane).remove(id);
        self.tabs.remove(&id).is_some()
    }

    /// 列出面板中的標籤。 / Returns the tabs contained in the given pane.
    pub fn tabs_in(&self, pane: Pane) -> impl Iterator<Item = &TabRecord> {
        let pane_state = self.pane_ref(pane);
        pane_state
            .tabs
            .iter()
            .filter_map(|id| self.tabs.get(id))
    }

    /// 回傳目前的啟動標籤。 / Returns the active tab for the pane.
    pub fn active_tab(&self, pane: Pane) -> Option<&TabRecord> {
        let pane_state = self.pane_ref(pane);
        pane_state
            .active
            .and_then(|id| self.tabs.get(&id))
    }

    /// 更新髒狀態。 / Marks a tab dirty or clean.
    pub fn set_dirty(&mut self, id: TabId, dirty: bool) -> bool {
        if let Some(record) = self.tabs.get_mut(&id) {
            record.is_dirty = dirty;
            true
        } else {
            false
        }
    }

    /// 判定某標籤位於何面板。 / Locates the pane containing the tab.
    pub fn locate_pane(&self, id: TabId) -> Option<Pane> {
        if self.panes[0].tabs.iter().any(|&tab| tab == id) {
            Some(Pane::Primary)
        } else if self.panes[1].tabs.iter().any(|&tab| tab == id) {
            Some(Pane::Secondary)
        } else {
            None
        }
    }

    fn pane_index(pane: Pane) -> usize {
        match pane {
            Pane::Primary => 0,
            Pane::Secondary => 1,
        }
    }

    fn other_pane(&self, pane: Pane) -> Pane {
        match pane {
            Pane::Primary => Pane::Secondary,
            Pane::Secondary => Pane::Primary,
        }
    }

    fn pane_mut(&mut self, pane: Pane) -> &mut PaneState {
        let index = Self::pane_index(pane);
        &mut self.panes[index]
    }

    fn pane_ref(&self, pane: Pane) -> &PaneState {
        let index = Self::pane_index(pane);
        &self.panes[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_move_and_clone_tabs() {
        let mut state = SplitViewState::default();
        let tab = state.open_tab(Pane::Primary, "main.rs", None);
        assert_eq!(state.locate_pane(tab), Some(Pane::Primary));

        assert!(state.move_to(tab, Pane::Secondary));
        assert_eq!(state.locate_pane(tab), Some(Pane::Secondary));

        let clone = state.clone_to_other(tab).unwrap();
        assert_eq!(state.locate_pane(clone), Some(Pane::Primary));
        assert_eq!(state.tabs_in(Pane::Primary).count(), 1);
        assert_eq!(state.tabs_in(Pane::Secondary).count(), 1);
    }

    #[test]
    fn close_tab_updates_active_state() {
        let mut state = SplitViewState::default();
        let t1 = state.open_tab(Pane::Primary, "a.txt", None);
        let t2 = state.open_tab(Pane::Primary, "b.txt", None);
        assert!(state.close_tab(t1));
        assert_eq!(
            state.active_tab(Pane::Primary).map(|tab| tab.id),
            Some(t2)
        );
    }
}
