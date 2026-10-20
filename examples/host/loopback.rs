//! Native loopback: the exact lab packet helpers, without radio or a device.
#[path = "../common/wire.rs"]
mod wire;
use nowctrl::{
    control::{Delivery, Pending, Request, VirtualNode},
    preset::{Axes, RcAction, RcState},
    protocol::{Control, MAX_FRAME_LEN},
};

fn main() {
    let mut packet = [0; MAX_FRAME_LEN];
    let command = Control::Toggle;
    let request = Request {
        node_id: wire::SWITCH_NODE_ID,
        request_id: 1,
        command,
    };
    let mut pending = Pending::new(request, 0).unwrap();
    let mut server = VirtualNode::new(wire::SWITCH_NODE_ID).unwrap();
    // The first response is deliberately dropped. A retry must not toggle again.
    for now in [0, 200] {
        assert_eq!(pending.poll(now), Ok(Delivery::Send(request)));
        let len = wire::switch_packet(command, 1, &mut packet).unwrap();
        let (request_id, action) = wire::switch_command(&packet[..len]).unwrap();
        let result = server
            .apply(wire::SWITCH_NODE_ID, request_id, action, now)
            .unwrap();
        assert!(server.output());
        let len = wire::ack_packet(1, result, &mut packet).unwrap();
        let (id, ack) = wire::switch_ack(&packet[..len]).unwrap();
        if now == 200 {
            assert!(pending.acknowledge(wire::SWITCH_NODE_ID, id, ack, 201));
        }
        println!(
            "switch server: output={} request={} retry={}",
            server.output(),
            id,
            now != 0
        );
    }
    assert!(matches!(pending.poll(201).unwrap(), Delivery::Complete(_)));
    println!("switch client: application ACK accepted; duplicate toggle did not reexecute");

    let mut rc = RcState::default();
    let len = wire::rc_packet(RcAction::Forward, 1, &mut packet).unwrap();
    let (seq, action) = wire::rc_update(&packet[..len]).unwrap();
    rc.update(seq, action, 0).unwrap();
    println!("RC server: {:?}", rc.advance(0).unwrap());
    assert_eq!(rc.advance(300), Ok(Axes::default()));
    println!("RC server: stopped at 300ms without a fresh update");
    println!("PASS: host loopback only; no radio, GPIO or device authentication was tested");
}
