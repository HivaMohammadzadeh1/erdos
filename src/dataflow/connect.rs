use std::sync::{Arc, Mutex};

use serde::Deserialize;

use crate::{
    dataflow::{
        graph::default_graph, operator::*, Data, StateT, State, Stream, StreamT, AppendableStateT
    },
    node::operator_executors::{
        OneInExecutor, OneInOneOutMessageProcessor, OneInTwoOutExecutor, OperatorExecutorT,
        ParallelOneInOneOutMessageProcessor, ParallelSinkMessageProcessor, SinkMessageProcessor,
        SourceExecutor, TwoInOneOutExecutor,
    },
    scheduler::channel_manager::ChannelManager,
    OperatorId,
};

pub fn connect_source<O, S, T>(
    operator_fn: impl Fn() -> O + Clone + Send + Sync + 'static,
    state_fn: impl Fn() -> S + Clone + Send + Sync + 'static,
    mut config: OperatorConfig,
) -> Stream<T>
where
    O: 'static + Source<S, T>,
    S: State,
    T: Data + for<'a> Deserialize<'a>,
{
    config.id = OperatorId::new_deterministic();
    let write_stream = Stream::new();

    let write_stream_ids = vec![write_stream.id()];

    let write_stream_ids_copy = write_stream_ids.clone();
    let config_copy = config.clone();
    let op_runner =
        move |channel_manager: Arc<Mutex<ChannelManager>>| -> Box<dyn OperatorExecutorT> {
            let mut channel_manager = channel_manager.lock().unwrap();

            let write_stream = channel_manager
                .get_write_stream(write_stream_ids_copy[0])
                .unwrap();

            let executor = SourceExecutor::new(
                config_copy.clone(),
                operator_fn.clone(),
                state_fn.clone(),
                write_stream,
            );

            Box::new(executor)
        };

    default_graph::add_operator(
        config.id,
        config.name,
        config.node_id,
        Vec::new(),
        write_stream_ids,
        op_runner,
    );
    default_graph::add_operator_stream(config.id, &write_stream);

    write_stream
}

pub fn connect_parallel_sink<O, S, T, U>(
    operator_fn: impl Fn() -> O + Clone + Send + Sync + 'static,
    state_fn: impl Fn() -> S + Clone + Send + Sync + 'static,
    mut config: OperatorConfig,
    read_stream: &impl StreamT<T>,
) where
    O: 'static + ParallelSink<S, T, U>,
    S: AppendableStateT<U>,
    T: Data + for<'a> Deserialize<'a>,
    U: 'static + Send + Sync,
{
    config.id = OperatorId::new_deterministic();

    let read_stream_ids = vec![read_stream.id()];

    let read_stream_ids_copy = read_stream_ids.clone();
    let config_copy = config.clone();

    let op_runner =
        move |channel_manager: Arc<Mutex<ChannelManager>>| -> Box<dyn OperatorExecutorT> {
            let mut channel_manager = channel_manager.lock().unwrap();

            let read_stream = channel_manager
                .take_read_stream(read_stream_ids_copy[0])
                .unwrap();

            Box::new(OneInExecutor::new(
                config_copy.clone(),
                Box::new(ParallelSinkMessageProcessor::new(
                    config_copy.clone(),
                    operator_fn.clone(),
                    state_fn.clone(),
                )),
                read_stream,
            ))
        };

    default_graph::add_operator(
        config.id,
        config.name,
        config.node_id,
        read_stream_ids,
        vec![],
        op_runner,
    );
}

pub fn connect_sink<O, S, T>(
    operator_fn: impl Fn() -> O + Clone + Send + Sync + 'static,
    state_fn: impl Fn() -> S + Clone + Send + Sync + 'static,
    mut config: OperatorConfig,
    read_stream: &impl StreamT<T>,
) where
    O: 'static + Sink<S, T>,
    S: StateT,
    T: Data + for<'a> Deserialize<'a>,
{
    config.id = OperatorId::new_deterministic();

    let read_stream_ids = vec![read_stream.id()];

    let read_stream_ids_copy = read_stream_ids.clone();
    let config_copy = config.clone();

    let op_runner =
        move |channel_manager: Arc<Mutex<ChannelManager>>| -> Box<dyn OperatorExecutorT> {
            let mut channel_manager = channel_manager.lock().unwrap();

            let read_stream = channel_manager
                .take_read_stream(read_stream_ids_copy[0])
                .unwrap();

            Box::new(OneInExecutor::new(
                config_copy.clone(),
                Box::new(SinkMessageProcessor::new(
                    config_copy.clone(),
                    operator_fn.clone(),
                    state_fn.clone(),
                )),
                read_stream,
            ))
        };

    default_graph::add_operator(
        config.id,
        config.name,
        config.node_id,
        read_stream_ids,
        vec![],
        op_runner,
    );
}

pub fn connect_parallel_only_one_in_one_out<O, S, T, U, V>(
    operator_fn: impl Fn() -> O + Clone + Send + Sync + 'static,
    state_fn: impl Fn() -> S + Clone + Send + Sync + 'static,
    mut config: OperatorConfig,
    read_stream: &impl StreamT<T>,
) -> Stream<U>
where
    O: 'static + ParallelOneInOneOut<S, T, U, V>,
    S: AppendableStateT<V>,
    T: Data + for<'a> Deserialize<'a>,
    U: Data + for<'a> Deserialize<'a>,
    V: 'static + Send + Sync,
{
    config.id = OperatorId::new_deterministic();
    let write_stream = Stream::new();

    let read_stream_ids = vec![read_stream.id()];
    let write_stream_ids = vec![write_stream.id()];

    let read_stream_ids_copy = read_stream_ids.clone();
    let write_stream_ids_copy = write_stream_ids.clone();
    let config_copy = config.clone();
    let op_runner =
        move |channel_manager: Arc<Mutex<ChannelManager>>| -> Box<dyn OperatorExecutorT> {
            let mut channel_manager = channel_manager.lock().unwrap();

            let read_stream = channel_manager
                .take_read_stream(read_stream_ids_copy[0])
                .unwrap();
            let write_stream = channel_manager
                .get_write_stream(write_stream_ids_copy[0])
                .unwrap();

            Box::new(OneInExecutor::new(
                config_copy.clone(),
                Box::new(ParallelOneInOneOutMessageProcessor::new(
                    config_copy.clone(),
                    operator_fn.clone(),
                    state_fn.clone(),
                    write_stream,
                )),
                read_stream,
            ))
        };

    default_graph::add_operator(
        config.id,
        config.name,
        config.node_id,
        read_stream_ids,
        write_stream_ids,
        op_runner,
    );
    default_graph::add_operator_stream(config.id, &write_stream);

    write_stream
}

pub fn connect_one_in_one_out<O, S, T, U>(
    operator_fn: impl Fn() -> O + Clone + Send + Sync + 'static,
    state_fn: impl Fn() -> S + Clone + Send + Sync + 'static,
    mut config: OperatorConfig,
    read_stream: &impl StreamT<T>,
) -> Stream<U>
where
    O: 'static + OneInOneOut<S, T, U>,
    S: StateT,
    T: Data + for<'a> Deserialize<'a>,
    U: Data + for<'a> Deserialize<'a>,
{
    config.id = OperatorId::new_deterministic();
    let write_stream = Stream::new();

    let read_stream_ids = vec![read_stream.id()];
    let write_stream_ids = vec![write_stream.id()];

    let read_stream_ids_copy = read_stream_ids.clone();
    let write_stream_ids_copy = write_stream_ids.clone();
    let config_copy = config.clone();

    let op_runner =
        move |channel_manager: Arc<Mutex<ChannelManager>>| -> Box<dyn OperatorExecutorT> {
            let mut channel_manager = channel_manager.lock().unwrap();

            let read_stream = channel_manager
                .take_read_stream(read_stream_ids_copy[0])
                .unwrap();
            let write_stream = channel_manager
                .get_write_stream(write_stream_ids_copy[0])
                .unwrap();

            Box::new(OneInExecutor::new(
                config_copy.clone(),
                Box::new(OneInOneOutMessageProcessor::new(
                    config_copy.clone(),
                    operator_fn.clone(),
                    state_fn.clone(),
                    write_stream,
                )),
                read_stream,
            ))
        };

    default_graph::add_operator(
        config.id,
        config.name,
        config.node_id,
        read_stream_ids,
        write_stream_ids,
        op_runner,
    );
    default_graph::add_operator_stream(config.id, &write_stream);

    write_stream
}

/*
pub fn connect_one_in_one_out<O, S, T, U>(
    operator_fn: impl Fn() -> O + Clone + Send + Sync + 'static,
    state_fn: impl Fn() -> S + Clone + Send + Sync + 'static,
    mut config: OperatorConfig,
    read_stream: &impl StreamT<T>,
) -> Stream<U>
where
    O: 'static + OneInOneOut<S, T, U>,
    S: State,
    T: Data + for<'a> Deserialize<'a>,
    U: Data + for<'a> Deserialize<'a>,
{
    config.id = OperatorId::new_deterministic();
    let write_stream = Stream::new();

    let read_stream_ids = vec![read_stream.id()];
    let write_stream_ids = vec![write_stream.id()];

    let read_stream_ids_copy = read_stream_ids.clone();
    let write_stream_ids_copy = write_stream_ids.clone();
    let config_copy = config.clone();
    let op_runner =
        move |channel_manager: Arc<Mutex<ChannelManager>>| -> Box<dyn OperatorExecutorT> {
            let mut channel_manager = channel_manager.lock().unwrap();

            let read_stream = channel_manager
                .take_read_stream(read_stream_ids_copy[0])
                .unwrap();
            let write_stream = channel_manager
                .get_write_stream(write_stream_ids_copy[0])
                .unwrap();

            let executor = OneInOneOutExecutor::new(
                config_copy.clone(),
                operator_fn.clone(),
                state_fn.clone(),
                read_stream,
                write_stream,
            );

            Box::new(executor)
        };

    default_graph::add_operator(
        config.id,
        config.name,
        config.node_id,
        read_stream_ids,
        write_stream_ids,
        op_runner,
    );
    default_graph::add_operator_stream(config.id, &write_stream);

    write_stream
}*/

pub fn connect_two_in_one_out<O, S, T, U, V>(
    operator_fn: impl Fn() -> O + Clone + Send + Sync + 'static,
    state_fn: impl Fn() -> S + Clone + Send + Sync + 'static,
    mut config: OperatorConfig,
    left_read_stream: &impl StreamT<T>,
    right_read_stream: &impl StreamT<U>,
) -> Stream<V>
where
    O: 'static + TwoInOneOut<S, T, U, V>,
    S: State,
    T: Data + for<'a> Deserialize<'a>,
    U: Data + for<'a> Deserialize<'a>,
    V: Data + for<'a> Deserialize<'a>,
{
    config.id = OperatorId::new_deterministic();
    let write_stream = Stream::new();

    let read_stream_ids = vec![left_read_stream.id(), right_read_stream.id()];
    let write_stream_ids = vec![write_stream.id()];

    let read_stream_ids_copy = read_stream_ids.clone();
    let write_stream_ids_copy = write_stream_ids.clone();
    let config_copy = config.clone();
    let op_runner =
        move |channel_manager: Arc<Mutex<ChannelManager>>| -> Box<dyn OperatorExecutorT> {
            let mut channel_manager = channel_manager.lock().unwrap();

            let left_read_stream = channel_manager
                .take_read_stream(read_stream_ids_copy[0])
                .unwrap();
            let right_read_stream = channel_manager
                .take_read_stream(read_stream_ids_copy[1])
                .unwrap();
            let write_stream = channel_manager
                .get_write_stream(write_stream_ids_copy[0])
                .unwrap();

            let executor = TwoInOneOutExecutor::new(
                config_copy.clone(),
                operator_fn.clone(),
                state_fn.clone(),
                left_read_stream,
                right_read_stream,
                write_stream,
            );

            Box::new(executor)
        };

    default_graph::add_operator(
        config.id,
        config.name,
        config.node_id,
        read_stream_ids,
        write_stream_ids,
        op_runner,
    );
    default_graph::add_operator_stream(config.id, &write_stream);

    write_stream
}

pub fn connect_one_in_two_out<O, S, T, U, V>(
    operator_fn: impl Fn() -> O + Clone + Send + Sync + 'static,
    state_fn: impl Fn() -> S + Clone + Send + Sync + 'static,
    mut config: OperatorConfig,
    read_stream: &impl StreamT<T>,
) -> (Stream<U>, Stream<V>)
where
    O: 'static + OneInTwoOut<S, T, U, V>,
    S: State,
    T: Data + for<'a> Deserialize<'a>,
    U: Data + for<'a> Deserialize<'a>,
    V: Data + for<'a> Deserialize<'a>,
{
    config.id = OperatorId::new_deterministic();
    let left_write_stream = Stream::new();
    let right_write_stream = Stream::new();

    let read_stream_ids = vec![read_stream.id()];
    let write_stream_ids = vec![left_write_stream.id(), right_write_stream.id()];

    let read_stream_ids_copy = read_stream_ids.clone();
    let write_stream_ids_copy = write_stream_ids.clone();
    let config_copy = config.clone();
    let op_runner =
        move |channel_manager: Arc<Mutex<ChannelManager>>| -> Box<dyn OperatorExecutorT> {
            let mut channel_manager = channel_manager.lock().unwrap();

            let read_stream = channel_manager
                .take_read_stream(read_stream_ids_copy[0])
                .unwrap();
            let left_write_stream = channel_manager
                .get_write_stream(write_stream_ids_copy[0])
                .unwrap();
            let right_write_stream = channel_manager
                .get_write_stream(write_stream_ids_copy[0])
                .unwrap();

            let executor = OneInTwoOutExecutor::new(
                config_copy.clone(),
                operator_fn.clone(),
                state_fn.clone(),
                read_stream,
                left_write_stream,
                right_write_stream,
            );

            Box::new(executor)
        };

    default_graph::add_operator(
        config.id,
        config.name,
        config.node_id,
        read_stream_ids,
        write_stream_ids,
        op_runner,
    );
    default_graph::add_operator_stream(config.id, &left_write_stream);
    default_graph::add_operator_stream(config.id, &right_write_stream);

    (left_write_stream, right_write_stream)
}
