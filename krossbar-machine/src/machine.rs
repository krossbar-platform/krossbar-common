pub struct Machine<State: Send, Ret = State> {
    states: Vec<fn(State) -> State>,
    ret: Option<fn(State) -> Ret>,
}

impl<State: Send, Ret> Machine<State, Ret> {}
