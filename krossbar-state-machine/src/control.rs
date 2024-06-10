/// Control enum returned for a machine stage
pub enum Control<State, Ret> {
    /// Loop the function one more time
    Loop(State),
    /// Go to the next machine stage
    Return(Ret),
}
