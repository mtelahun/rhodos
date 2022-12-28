use rand::{distributions::Alphanumeric, thread_rng, Rng};

pub mod get;
pub mod post;
pub mod post_form;

fn generate_random_data(len: usize) -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(len)
        .collect()
}
