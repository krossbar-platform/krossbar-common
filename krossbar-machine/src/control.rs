pub enum Control<State, Ret> {
    Loop(State),
    Return(Ret),
}
