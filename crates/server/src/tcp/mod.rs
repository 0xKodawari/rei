pub enum State {
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWiat,
    Closing,
    LastAck,
    TimeWait,
    Closed,
}
