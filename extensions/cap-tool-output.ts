/**
 * Cap tool result size sent to the model (LLM context), tighter than Pi defaults.
 * Optional edit-arg redaction + block for very large edit payloads.
 */
import type { AgentMessage, ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { formatSize, truncateHead } from "@earendil-works/pi-coding-agent";

type Limits = { maxLines: number; maxBytes: number };

const TOOL_RESULT_LIMITS: Record<string, Limits> = {
	read: { maxLines: 80, maxBytes: 8192 },
	bash: { maxLines: 120, maxBytes: 12_288 },
	grep: { maxLines: 60, maxBytes: 8192 },
	find: { maxLines: 60, maxBytes: 8192 },
	fffind: { maxLines: 40, maxBytes: 6144 },
	ffgs: { maxLines: 40, maxBytes: 6144 },
	ffgrep: { maxLines: 40, maxBytes: 6144 },
};

const DEFAULT_RESULT_LIMITS: Limits = { maxLines: 100, maxBytes: 10_240 };

const EDIT_FIELD_CONTEXT_LIMITS: Limits = { maxLines: 30, maxBytes: 1536 };
const EDIT_BLOCK_BYTES = 20_480;

const SKIP_RESULT_TOOLS = new Set(["edit", "write"]);

function byteLength(text: string): number {
	return Buffer.byteLength(text, "utf8");
}

function truncateText(text: string, limits: Limits, notice: string): string {
	const truncation = truncateHead(text, limits);
	if (!truncation.truncated) {
		return text;
	}
	const omittedLines = truncation.totalLines - truncation.outputLines;
	const omittedBytes = truncation.totalBytes - truncation.outputBytes;
	return (
		truncation.content +
		`\n\n[${notice}: ${truncation.outputLines}/${truncation.totalLines} lines` +
		` (${formatSize(truncation.outputBytes)} of ${formatSize(truncation.totalBytes)});` +
		` ${omittedLines} lines / ${formatSize(omittedBytes)} omitted.]`
	);
}

function editPayloadBytes(edits: unknown): number {
	if (!Array.isArray(edits)) {
		return 0;
	}
	let total = 0;
	for (const edit of edits) {
		if (!edit || typeof edit !== "object") {
			continue;
		}
		const record = edit as Record<string, unknown>;
		for (const key of ["oldText", "newText", "old_string", "new_string"]) {
			const value = record[key];
			if (typeof value === "string") {
				total += byteLength(value);
			}
		}
	}
	return total;
}

function redactEditEdits(edits: unknown): { edits: unknown; changed: boolean } {
	if (!Array.isArray(edits)) {
		return { edits, changed: false };
	}
	let changed = false;
	const next = edits.map((edit) => {
		if (!edit || typeof edit !== "object") {
			return edit;
		}
		const record = { ...(edit as Record<string, unknown>) };
		for (const key of ["oldText", "newText", "old_string", "new_string"]) {
			const value = record[key];
			if (typeof value === "string") {
				const nextValue = truncateText(
					value,
					EDIT_FIELD_CONTEXT_LIMITS,
					"Edit snippet capped in context",
				);
				if (nextValue !== value) {
					record[key] = nextValue;
					changed = true;
				}
			}
		}
		return record;
	});
	return { edits: next, changed };
}

function redactAssistantMessage(message: AgentMessage): AgentMessage {
	if (message.role !== "assistant" || !Array.isArray(message.content)) {
		return message;
	}

	let changed = false;
	const content = message.content.map((block) => {
		if (block.type !== "toolCall" || block.name !== "edit") {
			return block;
		}

		const args = block.arguments;
		if (!args || typeof args !== "object" || !Array.isArray(args.edits)) {
			return block;
		}

		const { edits: nextEdits, changed: editsChanged } = redactEditEdits(args.edits);
		if (!editsChanged) {
			return block;
		}

		changed = true;
		return {
			...block,
			arguments: { ...args, edits: nextEdits },
		};
	});

	return changed ? { ...message, content } : message;
}

export default function (pi: ExtensionAPI) {
	pi.on("context", async (event) => {
		let changed = false;
		const messages = event.messages.map((message) => {
			const next = redactAssistantMessage(message);
			if (next !== message) {
				changed = true;
			}
			return next;
		});
		return changed ? { messages } : undefined;
	});

	pi.on("tool_call", async (event) => {
		if (event.toolName !== "edit") {
			return;
		}

		const total = editPayloadBytes(event.input.edits);
		if (total > EDIT_BLOCK_BYTES) {
			return {
				block: true,
				reason:
					`Edit payload too large (${formatSize(total)}; limit ${formatSize(EDIT_BLOCK_BYTES)}). ` +
					`Split into multiple smaller edits.`,
			};
		}
	});

	pi.on("tool_result", async (event) => {
		if (SKIP_RESULT_TOOLS.has(event.toolName)) {
			return;
		}

		const limits = TOOL_RESULT_LIMITS[event.toolName] ?? DEFAULT_RESULT_LIMITS;
		let changed = false;

		const content = event.content.map((block) => {
			if (block.type !== "text" || !block.text) {
				return block;
			}

			const truncation = truncateHead(block.text, limits);
			if (!truncation.truncated) {
				return block;
			}

			changed = true;
			const omittedLines = truncation.totalLines - truncation.outputLines;
			const omittedBytes = truncation.totalBytes - truncation.outputBytes;
			const notice =
				`\n\n[Tool output capped for context: ${truncation.outputLines}/${truncation.totalLines} lines` +
				` (${formatSize(truncation.outputBytes)} of ${formatSize(truncation.totalBytes)});` +
				` ${omittedLines} lines / ${formatSize(omittedBytes)} omitted.` +
				` Use read offset/limit or narrower search.]`;

			return { ...block, text: truncation.content + notice };
		});

		if (!changed) {
			return;
		}

		return { content };
	});
}
