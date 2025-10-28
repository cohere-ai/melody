use crate::filter::{Filter, FilterImpl};
use crate::types::*;
use std::marker::PhantomData;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

pub struct StreamFilter<D: Send + 'static> {
    input_tx: Sender<Option<FullTextWithLogprobs>>,
    output_rx: Receiver<FilterOutput>,
    filter_handle: Option<thread::JoinHandle<()>>,
    _phantom: PhantomData<D>,
}

impl<D: Send + 'static> StreamFilter<D> {
    pub fn new(mut filter: FilterImpl) -> Self {
        let (input_tx, input_rx) = channel::<Option<FullTextWithLogprobs>>();
        let (output_tx, output_rx) = channel::<FilterOutput>();

        let filter_handle = thread::spawn(move || {
            while let Ok(Some(t)) = input_rx.recv() {
                let filter_outputs = filter.write_text(&t.text, t.logprobs);
                for output in filter_outputs {
                    if output_tx.send(output).is_err() {
                        break;
                    }
                }
            }

            // Flush partials when input is closed
            let outputs = filter.flush_partials();
            for output in outputs {
                let _ = output_tx.send(output);
            }
        });

        Self {
            input_tx,
            output_rx,
            filter_handle: Some(filter_handle),
            _phantom: PhantomData,
        }
    }

    pub fn read(&self) -> &Receiver<FilterOutput> {
        &self.output_rx
    }

    pub fn write(
        &self,
        _token: i64,
        _likelihood: Option<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // We need to decode the token first
        // For stream filter, the decoding happens in the caller
        // This is a simplified implementation
        Ok(())
    }

    pub fn write_decoded(&self, decoded_token: &str, logprobs: TokenIDsWithLogProb) {
        let _ = self.input_tx.send(Some(FullTextWithLogprobs {
            text: decoded_token.as_bytes().to_vec(),
            logprobs,
        }));
    }

    pub fn close(mut self) {
        let _ = self.input_tx.send(None);
        drop(self.input_tx);
        if let Some(handle) = self.filter_handle.take() {
            let _ = handle.join();
        }
    }
}
