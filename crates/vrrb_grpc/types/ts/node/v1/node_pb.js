// @generated by protoc-gen-es v1.2.0
// @generated from file node/v1/node.proto (package node.v1, syntax proto3)
/* eslint-disable */
// @ts-nocheck

import { proto3 } from "@bufbuild/protobuf";

/**
 * @generated from message node.v1.GetNodeTypeRequest
 */
export const GetNodeTypeRequest = proto3.makeMessageType(
  "node.v1.GetNodeTypeRequest",
  [],
);

/**
 * @generated from message node.v1.GetNodeTypeResponse
 */
export const GetNodeTypeResponse = proto3.makeMessageType(
  "node.v1.GetNodeTypeResponse",
  () => [
    { no: 1, name: "id", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 2, name: "result", kind: "scalar", T: 9 /* ScalarType.STRING */ },
  ],
);

/**
 * @generated from message node.v1.GetFullMempoolRequest
 */
export const GetFullMempoolRequest = proto3.makeMessageType(
  "node.v1.GetFullMempoolRequest",
  [],
);

/**
 * @generated from message node.v1.GetFullMempoolResponse
 */
export const GetFullMempoolResponse = proto3.makeMessageType(
  "node.v1.GetFullMempoolResponse",
  () => [
    { no: 1, name: "transaction_records", kind: "message", T: TransactionRecord, repeated: true },
  ],
);

/**
 * @generated from message node.v1.CreateTransactionRequest
 */
export const CreateTransactionRequest = proto3.makeMessageType(
  "node.v1.CreateTransactionRequest",
  () => [
    { no: 1, name: "timestamp", kind: "scalar", T: 3 /* ScalarType.INT64 */ },
    { no: 2, name: "sender_address", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 3, name: "sender_public_key", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 4, name: "receiver_address", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 5, name: "token", kind: "message", T: Token },
    { no: 6, name: "amount", kind: "scalar", T: 4 /* ScalarType.UINT64 */ },
    { no: 7, name: "signature", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 8, name: "validators", kind: "map", K: 9 /* ScalarType.STRING */, V: {kind: "scalar", T: 8 /* ScalarType.BOOL */} },
    { no: 9, name: "nonce", kind: "scalar", T: 4 /* ScalarType.UINT64 */ },
  ],
);

/**
 * @generated from message node.v1.TransactionRecord
 */
export const TransactionRecord = proto3.makeMessageType(
  "node.v1.TransactionRecord",
  () => [
    { no: 1, name: "id", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 2, name: "timestamp", kind: "scalar", T: 3 /* ScalarType.INT64 */ },
    { no: 3, name: "sender_address", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 4, name: "sender_public_key", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 5, name: "receiver_address", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 6, name: "token", kind: "message", T: Token },
    { no: 7, name: "amount", kind: "scalar", T: 4 /* ScalarType.UINT64 */ },
    { no: 8, name: "signature", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 9, name: "validators", kind: "map", K: 9 /* ScalarType.STRING */, V: {kind: "scalar", T: 8 /* ScalarType.BOOL */} },
    { no: 10, name: "nonce", kind: "scalar", T: 4 /* ScalarType.UINT64 */ },
  ],
);

/**
 * @generated from message node.v1.Token
 */
export const Token = proto3.makeMessageType(
  "node.v1.Token",
  () => [
    { no: 1, name: "name", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 2, name: "symbol", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 3, name: "decimals", kind: "scalar", T: 13 /* ScalarType.UINT32 */ },
  ],
);

