#![doc = include_str!("../README.md")]

extern crate gtk4 as gtk;
extern crate libadwaita as adw;

use std::{os::fd::OwnedFd, thread, time::Duration};

use adw::{gio, prelude::*};
use anyhow::{Context, Result};
use ashpd::{
    WindowIdentifier,
    desktop::{
        PersistMode,
        screencast::{CursorMode, Screencast, SourceType},
        screenshot::Screenshot,
    },
};
use glib::MainLoop;
use libspa::{pod::Pod, utils::Direction};
use pipewire::{
    properties::Properties,
    stream::StreamFlags,
    sys::{PW_KEY_MEDIA_CATEGORY, PW_KEY_MEDIA_ROLE, PW_KEY_MEDIA_TYPE},
};
use tokio::{sync::oneshot, task};
use wordbase_engine::Engine;

const APP_ID: &str = "io.github.aecsocket.Wordbase";

#[tokio::main]
async fn main() -> Result<glib::ExitCode> {
    let data_dir = wordbase_engine::data_dir().context("failed to get data directory")?;
    let engine = Engine::new(&data_dir)
        .await
        .context("failed to create engine")?;
    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_activate(move |app| {
        let window = adw::ApplicationWindow::new(app);
        window.present();

        glib::spawn_future_local(fun_thing(window.upcast()));
    });

    Ok(app.run())
}

async fn fun_thing(window: gtk::Window) -> Result<()> {
    let parent_window = WindowIdentifier::from_native(&window)
        .await
        .context("failed to get window identifier from window")?;
    println!("parent win = {parent_window:?}");

    let (send_streams_fd, recv_streams_fd) = oneshot::channel();
    let (send_node_id, recv_node_id) = oneshot::channel();

    thread::spawn(move || pipewire_thread(recv_streams_fd, recv_node_id).unwrap());

    let screencast = Screencast::new()
        .await
        .context("failed to create screencast proxy")?;

    let session = screencast
        .create_session()
        .await
        .context("failed to create screencast session")?;

    screencast
        .select_sources(
            &session,
            CursorMode::Hidden,
            SourceType::Window | SourceType::Monitor,
            false,
            None,
            PersistMode::DoNot,
        )
        .await
        .context("failed to select sources")?;

    let resp = screencast
        .start(&session, Some(&parent_window))
        .await
        .context("failed to start screencast")?
        .response()?;

    let streams_fd = screencast
        .open_pipe_wire_remote(&session)
        .await
        .context("failed to get PipeWire remote streams fd")?;
    _ = send_streams_fd.send(streams_fd);

    let stream = resp.streams().first().context("no streams")?;
    _ = send_node_id.send(stream.pipe_wire_node_id());

    Ok(())
}

fn pipewire_thread(
    recv_streams_fd: oneshot::Receiver<OwnedFd>,
    recv_node_id: oneshot::Receiver<u32>,
) -> Result<()> {
    let main_loop = pipewire::main_loop::MainLoop::new(None)?;
    let context = pipewire::context::Context::new(&main_loop)?;

    let streams_fd = recv_streams_fd.blocking_recv()?;
    let core = context.connect_fd(streams_fd, None)?;
    let _registry = core.get_registry()?;

    let node_id = recv_node_id.blocking_recv()?;
    println!("node id = {node_id}");

    let mut props = Properties::new();
    props.insert(PW_KEY_MEDIA_TYPE, "Video");
    props.insert(PW_KEY_MEDIA_CATEGORY, "Capture");
    props.insert(PW_KEY_MEDIA_ROLE, "Camera");

    let stream = pipewire::stream::Stream::new(&core, "test-stream", props)?;

    let mut params = [];
    stream.connect(
        Direction::Input,
        Some(node_id),
        StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
        &mut params,
    )?;
    stream.set_active(true)?;
    println!("made stream, state = {:?}", stream.state());

    let _listener = stream
        .add_local_listener::<()>()
        .state_changed(move |_stream, (), old, new| {
            println!("old = {old:?}");
            println!("new = {new:?}");
        })
        .process(move |stream, ()| {
            let mut buf = stream.dequeue_buffer().expect("Failed to dequeue buffer");
            let data = buf.data_mut().first().expect("No data found in buffer");
            let chunk = data.chunk();
            println!("chunk size = {}", chunk.size());
        })
        .register()?;

    // let _listener = registry
    //     .add_listener_local()
    //     .global(move |global| {
    //         if global.id == node_id {
    //             println!("new node {}", global.id);
    //         }
    //     })
    //     .register();

    main_loop.run();
    anyhow::Ok(())
}
