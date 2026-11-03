fn main() {
    #[cfg(feature = "lucille-validation")]
    locally_uncensored_lib::lucille_validation::run();
    #[cfg(not(feature = "lucille-validation"))]
    panic!("Explicit lucille-validation feature required");
}
