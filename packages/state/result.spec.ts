// A refused call's ApiError (#451): the daemon's reason when it sends one, else its
// error, else the status text — one rule for every client (the app's and the relay's).
import { describe, it, expect } from 'vitest';
import { failureFrom } from './result.js';

const refusal = (status: number, statusText: string, body: string) => new Response(body, { status, statusText });

describe('failureFrom · the refusal a person reads', () => {
  it('a coded refusal with a reason: the reason, with the body kept for the code', async () => {
    const body = { error: 'relay_conflict', message: 'the board is connected to another project' };
    const error = await failureFrom(refusal(409, 'Conflict', JSON.stringify(body)));
    expect(error).toEqual({ status: 409, message: 'the board is connected to another project', body });
  });

  it('a reason in error alone: the error', async () => {
    const error = await failureFrom(refusal(400, 'Bad Request', JSON.stringify({ error: 'name is required' })));
    expect(error).toEqual({ status: 400, message: 'name is required', body: { error: 'name is required' } });
  });

  it('a JSON body with neither: the status text, the body kept', async () => {
    const error = await failureFrom(refusal(404, 'Not Found', JSON.stringify({ board_ref: '7' })));
    expect(error).toEqual({ status: 404, message: 'Not Found', body: { board_ref: '7' } });
  });

  it('an empty error or message is not a reason', async () => {
    const error = await failureFrom(refusal(500, 'Internal Server Error', JSON.stringify({ error: '', message: '' })));
    expect(error.message).toBe('Internal Server Error');
  });

  it('a body that is not JSON: the status text, no body', async () => {
    expect(await failureFrom(refusal(502, 'Bad Gateway', 'upstream down'))).toEqual({ status: 502, message: 'Bad Gateway' });
  });

  it('a JSON body that is not an object: the status text, no body', async () => {
    expect(await failureFrom(refusal(500, 'Internal Server Error', '"boom"'))).toEqual({ status: 500, message: 'Internal Server Error' });
  });
});
