use std::collections::vec_deque::VecDeque;
use std::result as result;

pub trait TimeMachineState<F, R> {
    fn apply_forward(&mut self, delta: &F) -> R;
    fn apply_reverse(&mut self, delta: &R);
}

#[derive(Debug, PartialEq)]
pub enum Error<T> {
    TimeEvicted(T, T)
}

pub type Result<D, T> = result::Result<D, Error<T>>;

struct Timestamped<T, D> (T, D);

pub struct TimeMachine<S, F, R, T> {
    current: S,
    reverse: VecDeque<Timestamped<T, (F, R)>>,
    forward: Vec<Timestamped<T, F>>,
    oldest: Option<T>
}

impl <S, F, R, T> TimeMachine<S, F, R, T>
    where S: TimeMachineState<F, R>,
          T: PartialOrd + Copy {
    pub fn new(initial: S) -> TimeMachine<S, F, R, T> {
        TimeMachine {
            current: initial,
            reverse: VecDeque::new(),
            forward: Vec::new(),
            oldest: None
        }
    }

    pub fn change(&mut self, delta: F, at: T) -> Result<(), T> {
        try!(self.check_oldest(at));
        self.move_to(at);
        self.forward.push(Timestamped(at, delta));
        Ok(())
    }

    pub fn value_at(&mut self, at: T) -> Result<&S, T> {
        try!(self.check_oldest(at));
        self.move_to(at);
        Ok(&self.current)
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

        self.oldest = Some(until);
    }

    fn check_oldest(&self, at: T) -> Result<(), T> {
        match self.oldest {
            Some(i) => 
                if i > at {
                    Err(Error::TimeEvicted(at, i))
                } else {
                    Ok(())
                },
            None => Ok(())
        }
    }

    fn move_to(&mut self, at: T) {
        self.move_forward_to(at);
        self.move_backward_to(at);
    }

    fn move_backward_to(&mut self, at: T) {
        loop {
            match self.reverse.pop_back() {
                Some(Timestamped(time, (delta_f, delta_r))) => 
                    if time <= at {
                        self.reverse.push_back(Timestamped(time, (delta_f, delta_r)));
                        break;
                    } else {
                        self.current.apply_reverse(&delta_r);
                        self.forward.push(Timestamped(time, delta_f));
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
                        let new_delta = self.current.apply_forward(&delta);
                        self.reverse.push_back(Timestamped(time, (delta, new_delta)));
                    },
                None => break
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, TimeMachine, TimeMachineState};

    #[derive(Debug, PartialEq)]
    struct TestTimeMachineState(i32);
    
    enum TestTimeMachineDelta {
        Add(i32),
        Sub(i32),
        Mul(i32),
        Div(i32)
    }

    impl TestTimeMachineState {
        fn apply(&mut self, delta: &TestTimeMachineDelta) -> TestTimeMachineDelta {
            match *delta {
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
        fn apply_forward(&mut self, delta: &TestTimeMachineDelta) -> TestTimeMachineDelta {
            self.apply(delta)
        }

        fn apply_reverse(&mut self, delta: &TestTimeMachineDelta) {
            self.apply(delta);
        }
    }

    type TestTimeMachine = TimeMachine<TestTimeMachineState, TestTimeMachineDelta, TestTimeMachineDelta, u32>;

    fn assert_machine_success(m: &mut TestTimeMachine, at: u32, expected: i32) {
        let result = m.value_at(at).unwrap();
        assert_eq!(&TestTimeMachineState(expected), result);
    }

    fn assert_machine_failure(m: &mut TestTimeMachine, at: u32, expected: Error<u32>) {
        let result = m.value_at(at);
        assert_eq!(Some(expected), result.err());
    }

    #[test]
    fn forward_change() {
        let mut m = TestTimeMachine::new(TestTimeMachineState(5));
        m.change(TestTimeMachineDelta::Add(3), 1).unwrap();
        assert_machine_success(&mut m, 1, 8);
    }

    #[test]
    fn rewind() {
        let mut m = TestTimeMachine::new(TestTimeMachineState(5));
        m.change(TestTimeMachineDelta::Add(3), 1).unwrap();
        assert_machine_success(&mut m, 0, 5);
    }

    #[test]
    fn move_around() {
        let mut m = TestTimeMachine::new(TestTimeMachineState(5));
        m.change(TestTimeMachineDelta::Add(3), 1).unwrap();
        m.change(TestTimeMachineDelta::Mul(4), 10).unwrap();
        m.change(TestTimeMachineDelta::Sub(2), 11).unwrap();
        m.change(TestTimeMachineDelta::Div(5), 20).unwrap();
        assert_machine_success(&mut m, 25, 6);
        assert_machine_success(&mut m, 10, 32);
        assert_machine_success(&mut m, 0, 5);
        assert_machine_success(&mut m, 15, 30);
        assert_machine_success(&mut m, 11, 30);
        assert_machine_success(&mut m, 8, 8);
        assert_machine_success(&mut m, 1, 8);
    }

    #[test]
    fn change_in_middle() {
        let mut m = TestTimeMachine::new(TestTimeMachineState(5));
        m.change(TestTimeMachineDelta::Add(3), 1).unwrap();
        m.change(TestTimeMachineDelta::Add(5), 10).unwrap();
        assert_machine_success(&mut m, 5, 8);
        assert_machine_success(&mut m, 10, 13);

        m.change(TestTimeMachineDelta::Mul(2), 5).unwrap();
        assert_machine_success(&mut m, 5, 16);
        assert_machine_success(&mut m, 10, 21);
    }

    #[test]
    fn test_forget_ancient_history() {
        let mut m = TestTimeMachine::new(TestTimeMachineState(5));
        m.change(TestTimeMachineDelta::Add(3), 1).unwrap();
        m.change(TestTimeMachineDelta::Mul(2), 2).unwrap();
        m.change(TestTimeMachineDelta::Add(2), 3).unwrap();
        m.change(TestTimeMachineDelta::Sub(10), 4).unwrap();
        m.forget_ancient_history(3);

        assert_machine_failure(&mut m, 1, Error::TimeEvicted(1, 3));
        assert_machine_failure(&mut m, 2, Error::TimeEvicted(2, 3));
        assert_machine_success(&mut m, 3, 18);
        assert_machine_success(&mut m, 4, 8);
    }
}
