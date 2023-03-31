use web_sys::Worker;

// Copied from https://github.com/thedodd/trunk/blob/master/examples/webworker/src/bin/app.rs
fn worker_new(name: &str) -> Worker {
    use js_sys::Array;
    use web_sys::{Blob, BlobPropertyBag, Url};

    let origin = web_sys::window()
        .expect("window to be available")
        .location()
        .origin()
        .expect("origin to be available");

    let script = Array::new();
    script.push(
        &format!(r#"importScripts("{origin}/{name}.js");wasm_bindgen("{origin}/{name}_bg.wasm");"#)
            .into(),
    );

    let blob = Blob::new_with_str_sequence_and_options(
        &script,
        BlobPropertyBag::new().type_("text/javascript"),
    )
    .expect("blob creation succeeds");

    let url = Url::create_object_url_with_blob(&blob).expect("url creation succeeds");

    Worker::new(&url).expect("failed to spawn worker")
}

fn main() {
    use wasm_bindgen::JsCast;
    use web_sys::HtmlCanvasElement;

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();

    // Create new canvas element and attach it to document.
    let element = document.create_element("canvas").unwrap();
    let canvas: HtmlCanvasElement = element.dyn_into().unwrap();
    // Bevy expects viewport of this size.
    canvas.set_width(1280);
    canvas.set_height(720);

    body.append_child(&canvas).unwrap();

    // We cannot pass canvas element to worker directly, instead we have to convert it to OffscreenCanvas.
    let offscreen_canvas = canvas.transfer_control_to_offscreen().unwrap();

    // Adapted from https://github.com/thedodd/trunk/blob/master/examples/webworker/src/bin/app.rs
    {
        use wasm_bindgen::prelude::Closure;
        use web_sys::MessageEvent;

        let worker = worker_new("bevy_worker");

        let onmessage = {
            let worker = worker.clone();

            Closure::wrap(Box::new(move |_: MessageEvent| {
                use js_sys::Array;

                let msg = offscreen_canvas.clone();

                let transfer = {
                    let r = Array::new();
                    r.push(&offscreen_canvas.clone().into());
                    r
                };

                // OffscreenCanvas is transferrable object.
                // Somewhat confusingly, this means we need to pass it twice:
                // once as part of message, and other time inside transfer *array*.
                // Otherwise JS runtime will panic.
                worker
                    .post_message_with_transfer(&msg.into(), &transfer.into())
                    .expect("sending message to succeed");
            }) as Box<dyn Fn(MessageEvent)>)
        };

        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();
    }
}
