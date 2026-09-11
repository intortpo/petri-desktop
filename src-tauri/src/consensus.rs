#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub text: String,
    pub score: f32,
    pub backend: String,
}

/// Pick the highest-scoring candidate. Not first-finisher: index 0 can lose.
pub fn pick_consensus(cands: &[Candidate]) -> String {
    if cands.is_empty() {
        return String::new();
    }
    cands
        .iter()
        .max_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.backend.cmp(&b.backend))
        })
        .map(|c| c.text.clone())
        .unwrap_or_default()
}

pub fn dispatch_concurrent<F, G>(prompt: &str, backend_a: F, backend_b: G) -> String
where
    F: FnOnce(&str) -> Candidate + Send,
    G: FnOnce(&str) -> Candidate + Send,
{
    std::thread::scope(|s| {
        let ha = s.spawn(|| backend_a(prompt));
        let hb = s.spawn(|| backend_b(prompt));
        let a = ha.join().expect("backend a");
        let b = hb.join().expect("backend b");
        pick_consensus(&[a, b])
    })
}

/// Peer backend used when a second live model is unavailable.
pub fn critic_backend(prompt: &str) -> Candidate {
    let text = format!("[critic] {}", prompt.trim());
    Candidate {
        text,
        score: 0.35,
        backend: "critic".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_does_not_blindly_take_first() {
        let first = Candidate {
            text: "first-finisher garbage".into(),
            score: 0.2,
            backend: "fast".into(),
        };
        let better = Candidate {
            text: "agreed better answer".into(),
            score: 0.91,
            backend: "slow".into(),
        };
        let got = pick_consensus(&[first.clone(), better.clone()]);
        assert_eq!(got, "agreed better answer");
        assert_ne!(got, first.text);
    }

    #[test]
    fn concurrent_dispatch_uses_picker() {
        let out = dispatch_concurrent(
            "q",
            |_| Candidate {
                text: "nope".into(),
                score: 0.1,
                backend: "a".into(),
            },
            |_| Candidate {
                text: "yes-agreed".into(),
                score: 0.8,
                backend: "b".into(),
            },
        );
        assert_eq!(out, "yes-agreed");
    }
}
