/**
 * Welcome to Cloudflare Workers! This is your first worker.
 *
 * - Run `npm run dev` in your terminal to start a development server
 * - Open a browser tab at http://localhost:8787/ to see your worker in action
 * - Run `npm run deploy` to publish your worker
 *
 * Bind resources to your worker in `wrangler.jsonc`. After adding bindings, a type definition for the
 * `Env` object can be regenerated with `npm run cf-typegen`.
 *
 * Learn more at https://developers.cloudflare.com/workers/
 */


export default {
	async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
		// get response from origin
		const response = await fetch(request);

		// should we track this request?
		const url = new URL(request.url);
		if (shouldTrack(url)) {
			// build payload and send async
			ctx.waitUntil((async () => {
				const event = await buildEvent(request);
				await sendEvent(event, env)
			})());
		}

		return response;
	}
}

function shouldTrack(url: URL): boolean {
	return url.pathname.endsWith('/') || url.pathname === '/index.xml';
}

interface AnalyticsEvent {
	url: string;
	referer: string | null;
	userAgent: string | null;
	country: string | null;
	city: string | null;
	timezone: string | null;
	timestamp: number;
	visitorHash: string;
}

// Extend standard Request interface to include Cloudflare specific properties
interface CloudflareRequest extends Request {
	cf?: {
		country?: string;
		city?: string;
		timezone?: string;
	};
}

function buildEvent(request: CloudflareRequest): AnalyticsEvent {
	return {
		// request.url contains the full URL (path + querystring)
		url: request.url,

		// note the HTTP misspelling 'referer'
		referer: request.headers.get('referer'),

		userAgent: request.headers.get('user-agent'),

		// Accessing Cloudflare propierties with optional chaining and null fallbacks
		country: request.cf?.country ?? null,
		city: request.cf?.city ?? null,
		timezone: request.cf?.timezone ?? null,

		// current time in milliseconds
		timestamp: Date.now(),
	};
}

interface Env {
	ANALYTICS_ENDPOINT: string;
	// Add other bindings here if needed (KV, D4, etc.)
}

async function sendEvent(event: AnalyticsEvent, env: Env): Promise<void> {
	try {
		const response = await fetch(env.ANALYTICS_ENDPOINT, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify(event),
		});

		// optional: log if the external service returns an error
		if (!response.ok) {
			console.error(
				`Analytics send failed: ${response.status} ${response.statusText}`
			);
		}
	} catch (error) {
		// log network errors (DNS failure, connection refused, etc.)
		console.error('Analytics send error: ', error);
	}
}

async function hashVisitorId(request: Request): Promise<string> {
	const ip = request.headers.get('cf-connecting-ip')  || 'unknown';

	// daily rotation: same IP gets a different hash each day
	const date = new Date().toISOString().slice(0,10);
	const raw = `${ip}:${date}`;
	const buffer = await crypto.subtle.digest(
		'SHA-256',
		new TextEncoder().encode(raw)
	);
	// convert to hex string
	const hashArray = Array.from(new Uint8Array(buffer));
	return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}
