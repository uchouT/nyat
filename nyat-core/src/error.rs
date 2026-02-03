use error_set::error_set;
// TODO: very awful error handling
error_set!(
    StunError := {
        MessageBuild,
    }
    pub Error := StunError || {
    Io(std::io::Error),
    DNS,
    }
);

impl From<stun::Error> for StunError {
    fn from(_value: stun::Error) -> Self {
        StunError::MessageBuild
    }
}
