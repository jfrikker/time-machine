use std::collections::vec_deque::VecDeque;

pub trait TimeMachineState<F, R> {
    fn apply_forward(&mut self, delta: F) -> R;
    fn apply_reverse(&mut self, delta: R) -> F;
}

struct Timestamped<T, D> (T, D);

pub struct TimeMachine<S, F, R, T> {
    current: S,
    reverse: VecDeque<Timestamped<T, R>>,
    forward: Vec<Timestamped<T, F>>
}

impl <S, F, R, T> TimeMachine<S, F, R, T>
    where S: TimeMachineState<F, R>,
          T: PartialOrd + Copy {
    pub fn new(initial: S) -> TimeMachine<S, F, R, T> {
        TimeMachine {
            current: initial,
            reverse: VecDeque::new(),
            forward: Vec::new()
        }
    }

    pub fn change(&mut self, delta: F, at: T) {
        self.move_to(at);
        self.forward.push(Timestamped(at, delta));
    }

    pub fn value_at(&mut self, at: T) -> &S {
        self.move_to(at);
        &self.current
    }

    pub fn forget_ancient_history(&mut self, until: T) {
        self.move_forward_to(until);

        loop {
            match self.reverse.pop_front() {
                Some(Timestamped(time, delta)) =>
                    if time >= until {
                        self.reverse.push_front(Timestamped(time, delta));
                        break;
                    },
                None => break
            }
        }
    }

    fn move_to(&mut self, at: T) {
        self.move_forward_to(at);
        self.move_backward_to(at);
    }

    fn move_backward_to(&mut self, at: T) {
        loop {
            match self.reverse.pop_back() {
                Some(Timestamped(time, delta)) => 
                    if time <= at {
                        self.reverse.push_back(Timestamped(time, delta));
                        break;
                    } else {
                        let new_delta = self.current.apply_reverse(delta);
                        self.forward.push(Timestamped(time, new_delta));
                    },
                None => break
            }
        }
    }

    fn move_forward_to(&mut self, at: T) {
        loop {
            match self.forward.pop() {
                Some(Timestamped(time, delta)) => 
                    if time > at {
                        self.forward.push(Timestamped(time, delta));
                        break;
                    } else {
                        let new_delta = self.current.apply_forward(delta);
                        self.reverse.push_back(Timestamped(time, new_delta));
                    },
                None => break
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TimeMachine, TimeMachineState};

    #[derive(Debug, PartialEq)]
    struct TestTimeMachineState(i32);
    
    enum TestTimeMachineDelta {
        Add(i32),
        Sub(i32),
        Mul(i32),
        Div(i32)
    }

    impl TestTimeMachineState {
        fn apply(&mut self, delta: TestTimeMachineDelta) -> TestTimeMachineDelta {
            match delta {
                TestTimeMachineDelta::Add(i) => {
                    self.0 += i;
                    TestTimeMachineDelta::Sub(i)
                },
                TestTimeMachineDelta::Sub(i) => {
                    self.0 -= i;
                    TestTimeMachineDelta::Add(i)
                },
                TestTimeMachineDelta::Mul(i) => {
                    self.0 *= i;
                    TestTimeMachineDelta::Div(i)
                },
                TestTimeMachineDelta::Div(i) => {
                    self.0 /= i;
                    TestTimeMachineDelta::Mul(i)
                },
            }
        }
    }

    impl TimeMachineState<TestTimeMachineDelta, TestTimeMachineDelta> for TestTimeMachineState {
        fn apply_forward(&mut self, delta: TestTimeMachineDelta) -> TestTimeMachineDelta {
            self.apply(delta)
        }

        fn apply_reverse(&mut self, delta: TestTimeMachineDelta) -> TestTimeMachineDelta {
            self.apply(delta)
        }
    }

    type TestTimeMachine = TimeMachine<TestTimeMachineState, TestTimeMachineDelta, TestTimeMachineDelta, u32>;

    #[test]
    fn forward_change() {
        let mut m = TestTimeMachine::new(TestTimeMachineState(5));
        m.change(TestTimeMachineDelta::Add(3), 1);
        assert_eq!(TestTimeMachineState(8), *m.value_at(1));
    }

    #[test]
    fn rewind() {
        let mut m = TestTimeMachine::new(TestTimeMachineState(5));
        m.change(TestTimeMachineDelta::Add(3), 1);
        assert_eq!(TestTimeMachineState(5), *m.value_at(0));
    }

    #[test]
    fn move_around() {
        let mut m = TestTimeMachine::new(TestTimeMachineState(5));
        m.change(TestTimeMachineDelta::Add(3), 1);
        m.change(TestTimeMachineDelta::Mul(4), 10);
        m.change(TestTimeMachineDelta::Sub(2), 11);
        m.change(TestTimeMachineDelta::Div(5), 20);
        assert_eq!(TestTimeMachineState(6), *m.value_at(25));
        assert_eq!(TestTimeMachineState(32), *m.value_at(10));
        assert_eq!(TestTimeMachineState(5), *m.value_at(0));
        assert_eq!(TestTimeMachineState(30), *m.value_at(15));
        assert_eq!(TestTimeMachineState(30), *m.value_at(11));
        assert_eq!(TestTimeMachineState(8), *m.value_at(8));
        assert_eq!(TestTimeMachineState(8), *m.value_at(1));
    }

    #[test]
    fn change_in_middle() {
        let mut m = TestTimeMachine::new(TestTimeMachineState(5));
        m.change(TestTimeMachineDelta::Add(3), 1);
        m.change(TestTimeMachineDelta::Add(5), 10);
        assert_eq!(TestTimeMachineState(8), *m.value_at(5));
        assert_eq!(TestTimeMachineState(13), *m.value_at(10));

        m.change(TestTimeMachineDelta::Mul(2), 5);
        assert_eq!(TestTimeMachineState(16), *m.value_at(5));
        assert_eq!(TestTimeMachineState(21), *m.value_at(10));
    }

    #[test]
    fn test_forget_ancient_history() {
        let mut m = TestTimeMachine::new(TestTimeMachineState(5));
        m.change(TestTimeMachineDelta::Add(3), 1);
        m.change(TestTimeMachineDelta::Mul(2), 2);
        m.change(TestTimeMachineDelta::Add(2), 3);
        m.change(TestTimeMachineDelta::Sub(10), 4);
        m.forget_ancient_history(3);

        assert_eq!(TestTimeMachineState(16), *m.value_at(1));
        assert_eq!(TestTimeMachineState(16), *m.value_at(2));
        assert_eq!(TestTimeMachineState(18), *m.value_at(3));
        assert_eq!(TestTimeMachineState(8), *m.value_at(4));
    }
}
