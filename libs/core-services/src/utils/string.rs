use validator::ValidateEmail;

pub trait StringExt {
    fn is_email(&self) -> bool;
}

impl StringExt for String {
    fn is_email(&self) -> bool {
        self.validate_email()
    }
}
