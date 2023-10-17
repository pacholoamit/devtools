// @generated by protobuf-ts 2.9.1
// @generated from protobuf file "logs.proto" (package "rs.devtools.logs", syntax proto3)
// tslint:disable
import type { BinaryWriteOptions } from "@protobuf-ts/runtime";
import type { IBinaryWriter } from "@protobuf-ts/runtime";
import { WireType } from "@protobuf-ts/runtime";
import type { BinaryReadOptions } from "@protobuf-ts/runtime";
import type { IBinaryReader } from "@protobuf-ts/runtime";
import { UnknownFieldHandler } from "@protobuf-ts/runtime";
import type { PartialMessage } from "@protobuf-ts/runtime";
import { reflectionMergePartial } from "@protobuf-ts/runtime";
import { MESSAGE_TYPE } from "@protobuf-ts/runtime";
import { MessageType } from "@protobuf-ts/runtime";
import { Timestamp } from "./google/protobuf/timestamp";
import { Field } from "./common";
import { MetaId } from "./common";
import { SpanId } from "./common";
/**
 * @generated from protobuf message rs.devtools.logs.Update
 */
export interface Update {
    /**
     * A list of log events that happened since the last update.
     *
     * @generated from protobuf field: repeated rs.devtools.logs.LogEvent log_events = 1;
     */
    logEvents: LogEvent[];
    /**
     * A count of how many log events were dropped because
     * the event buffer was at capacity.
     *
     * If everything is working correctly, this should be 0. If this
     * number is greater than zero this indicates the event buffers capacity
     * should be increased or the publish interval decreased.
     *
     * @generated from protobuf field: uint64 dropped_events = 2;
     */
    droppedEvents: bigint;
}
/**
 * @generated from protobuf message rs.devtools.logs.LogEvent
 */
export interface LogEvent {
    /**
     * The main message body of the log.
     *
     * @generated from protobuf field: string message = 1;
     */
    message: string;
    /**
     * Log events can happen inside of spans and if they do, this field will indicate which span it was.
     *
     * @generated from protobuf field: optional rs.devtools.common.SpanId parent = 2;
     */
    parent?: SpanId;
    /**
     * Identifier for metadata describing static characteristics of all spans originating
     * from that call site, such as its name, source code location, verbosity level, and
     * the names of its fields.
     *
     * @generated from protobuf field: rs.devtools.common.MetaId metadata_id = 3;
     */
    metadataId?: MetaId;
    /**
     * User-defined key-value pairs of arbitrary data associated with the event.
     *
     * @generated from protobuf field: repeated rs.devtools.common.Field fields = 4;
     */
    fields: Field[];
    /**
     * Timestamp for the log event.
     *
     * @generated from protobuf field: google.protobuf.Timestamp at = 5;
     */
    at?: Timestamp;
}
// @generated message type with reflection information, may provide speed optimized methods
class Update$Type extends MessageType<Update> {
    constructor() {
        super("rs.devtools.logs.Update", [
            { no: 1, name: "log_events", kind: "message", repeat: 1 /*RepeatType.PACKED*/, T: () => LogEvent },
            { no: 2, name: "dropped_events", kind: "scalar", T: 4 /*ScalarType.UINT64*/, L: 0 /*LongType.BIGINT*/ }
        ]);
    }
    create(value?: PartialMessage<Update>): Update {
        const message = { logEvents: [], droppedEvents: 0n };
        globalThis.Object.defineProperty(message, MESSAGE_TYPE, { enumerable: false, value: this });
        if (value !== undefined)
            reflectionMergePartial<Update>(this, message, value);
        return message;
    }
    internalBinaryRead(reader: IBinaryReader, length: number, options: BinaryReadOptions, target?: Update): Update {
        let message = target ?? this.create(), end = reader.pos + length;
        while (reader.pos < end) {
            let [fieldNo, wireType] = reader.tag();
            switch (fieldNo) {
                case /* repeated rs.devtools.logs.LogEvent log_events */ 1:
                    message.logEvents.push(LogEvent.internalBinaryRead(reader, reader.uint32(), options));
                    break;
                case /* uint64 dropped_events */ 2:
                    message.droppedEvents = reader.uint64().toBigInt();
                    break;
                default:
                    let u = options.readUnknownField;
                    if (u === "throw")
                        throw new globalThis.Error(`Unknown field ${fieldNo} (wire type ${wireType}) for ${this.typeName}`);
                    let d = reader.skip(wireType);
                    if (u !== false)
                        (u === true ? UnknownFieldHandler.onRead : u)(this.typeName, message, fieldNo, wireType, d);
            }
        }
        return message;
    }
    internalBinaryWrite(message: Update, writer: IBinaryWriter, options: BinaryWriteOptions): IBinaryWriter {
        /* repeated rs.devtools.logs.LogEvent log_events = 1; */
        for (let i = 0; i < message.logEvents.length; i++)
            LogEvent.internalBinaryWrite(message.logEvents[i], writer.tag(1, WireType.LengthDelimited).fork(), options).join();
        /* uint64 dropped_events = 2; */
        if (message.droppedEvents !== 0n)
            writer.tag(2, WireType.Varint).uint64(message.droppedEvents);
        let u = options.writeUnknownFields;
        if (u !== false)
            (u == true ? UnknownFieldHandler.onWrite : u)(this.typeName, message, writer);
        return writer;
    }
}
/**
 * @generated MessageType for protobuf message rs.devtools.logs.Update
 */
export const Update = new Update$Type();
// @generated message type with reflection information, may provide speed optimized methods
class LogEvent$Type extends MessageType<LogEvent> {
    constructor() {
        super("rs.devtools.logs.LogEvent", [
            { no: 1, name: "message", kind: "scalar", T: 9 /*ScalarType.STRING*/ },
            { no: 2, name: "parent", kind: "message", T: () => SpanId },
            { no: 3, name: "metadata_id", kind: "message", T: () => MetaId },
            { no: 4, name: "fields", kind: "message", repeat: 1 /*RepeatType.PACKED*/, T: () => Field },
            { no: 5, name: "at", kind: "message", T: () => Timestamp }
        ]);
    }
    create(value?: PartialMessage<LogEvent>): LogEvent {
        const message = { message: "", fields: [] };
        globalThis.Object.defineProperty(message, MESSAGE_TYPE, { enumerable: false, value: this });
        if (value !== undefined)
            reflectionMergePartial<LogEvent>(this, message, value);
        return message;
    }
    internalBinaryRead(reader: IBinaryReader, length: number, options: BinaryReadOptions, target?: LogEvent): LogEvent {
        let message = target ?? this.create(), end = reader.pos + length;
        while (reader.pos < end) {
            let [fieldNo, wireType] = reader.tag();
            switch (fieldNo) {
                case /* string message */ 1:
                    message.message = reader.string();
                    break;
                case /* optional rs.devtools.common.SpanId parent */ 2:
                    message.parent = SpanId.internalBinaryRead(reader, reader.uint32(), options, message.parent);
                    break;
                case /* rs.devtools.common.MetaId metadata_id */ 3:
                    message.metadataId = MetaId.internalBinaryRead(reader, reader.uint32(), options, message.metadataId);
                    break;
                case /* repeated rs.devtools.common.Field fields */ 4:
                    message.fields.push(Field.internalBinaryRead(reader, reader.uint32(), options));
                    break;
                case /* google.protobuf.Timestamp at */ 5:
                    message.at = Timestamp.internalBinaryRead(reader, reader.uint32(), options, message.at);
                    break;
                default:
                    let u = options.readUnknownField;
                    if (u === "throw")
                        throw new globalThis.Error(`Unknown field ${fieldNo} (wire type ${wireType}) for ${this.typeName}`);
                    let d = reader.skip(wireType);
                    if (u !== false)
                        (u === true ? UnknownFieldHandler.onRead : u)(this.typeName, message, fieldNo, wireType, d);
            }
        }
        return message;
    }
    internalBinaryWrite(message: LogEvent, writer: IBinaryWriter, options: BinaryWriteOptions): IBinaryWriter {
        /* string message = 1; */
        if (message.message !== "")
            writer.tag(1, WireType.LengthDelimited).string(message.message);
        /* optional rs.devtools.common.SpanId parent = 2; */
        if (message.parent)
            SpanId.internalBinaryWrite(message.parent, writer.tag(2, WireType.LengthDelimited).fork(), options).join();
        /* rs.devtools.common.MetaId metadata_id = 3; */
        if (message.metadataId)
            MetaId.internalBinaryWrite(message.metadataId, writer.tag(3, WireType.LengthDelimited).fork(), options).join();
        /* repeated rs.devtools.common.Field fields = 4; */
        for (let i = 0; i < message.fields.length; i++)
            Field.internalBinaryWrite(message.fields[i], writer.tag(4, WireType.LengthDelimited).fork(), options).join();
        /* google.protobuf.Timestamp at = 5; */
        if (message.at)
            Timestamp.internalBinaryWrite(message.at, writer.tag(5, WireType.LengthDelimited).fork(), options).join();
        let u = options.writeUnknownFields;
        if (u !== false)
            (u == true ? UnknownFieldHandler.onWrite : u)(this.typeName, message, writer);
        return writer;
    }
}
/**
 * @generated MessageType for protobuf message rs.devtools.logs.LogEvent
 */
export const LogEvent = new LogEvent$Type();