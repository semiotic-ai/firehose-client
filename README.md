# Firehose Rust Client

Extract blocks from [StreamingFast Firehose](https://firehose.streamingfast.io/)
programatically in Rust.

## gRPC Endpoints

### Env Vars

Use environment variables to provide Firehose Ethereum and Firehose
Beacon providers of your choice.

To do this, place a `.env` file in the root of the `veemon` repository. 
See the `.env.example` file, also in the root of this repository, for what
you'll need, depending on your requirements. You may have access to a
Firehose provider and/or may need to use a provider. For example, you can
use [Pinax](https://app.pinax.network/) as an endpoint provider.

## firehose-ethereum and firehose-beacon gRPC

### proto files

We use the following protobuffers developed by Streamingfast via our
[`firehose-protos` crate](./../firehose-protos/README.md):

See the [`firehose-protos` docs](./../firehose-protos/README.md) for information
on these protobuffers and links to the different repos that they've come from.

Also check out [`forrestrie`](./../forrestrie/protos/README.md) for 
Rust-compiled protobuffer Beacon block implementations.

### gRPC Service Examples

If you're looking to quickly explore what the Firehose API offers,
you can use the `grpcurl` tool to test your connection, verify your
API token, and get a list of available gRPC services. This is a great
way to interact with the API without writing any code.

To get started, you can run the following command to retrieve the
service descriptions from the Firehose gRPC server:

```bash
grpcurl -plaintext <your-grpc-service>:<port> describe
```

Below is an example of the service descriptions you might see when
querying the Firehose server:

```terminal
grpc.health.v1.Health is a service:
service Health {
  rpc Check ( .grpc.health.v1.HealthCheckRequest ) returns ( .grpc.health.v1.HealthCheckResponse );
  rpc Watch ( .grpc.health.v1.HealthCheckRequest ) returns ( stream .grpc.health.v1.HealthCheckResponse );
}

grpc.reflection.v1.ServerReflection is a service:
service ServerReflection {
  rpc ServerReflectionInfo ( stream .grpc.reflection.v1.ServerReflectionRequest ) returns ( stream .grpc.reflection.v1.ServerReflectionResponse );
}

grpc.reflection.v1alpha.ServerReflection is a service:
service ServerReflection {
  rpc ServerReflectionInfo ( stream .grpc.reflection.v1alpha.ServerReflectionRequest ) returns ( stream .grpc.reflection.v1alpha.ServerReflectionResponse );
}

sf.firehose.v1.Stream is a service:
service Stream {
  rpc Blocks ( .sf.firehose.v1.Request ) returns ( stream .sf.firehose.v1.Response );
}

sf.firehose.v2.Fetch is a service:
service Fetch {
  rpc Block ( .sf.firehose.v2.SingleBlockRequest ) returns ( .sf.firehose.v2.SingleBlockResponse );
}

sf.firehose.v2.Stream is a service:
service Stream {
  rpc Blocks ( .sf.firehose.v2.Request ) returns ( stream .sf.firehose.v2.Response );
}
```
