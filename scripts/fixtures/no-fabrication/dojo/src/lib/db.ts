// Fixture: dōjō supabase reads. T5 is an absolute zero, not a ratchet.
export async function members(supabase: Client) {
	const { data } = await supabase.from('members').select('*'); // expect: T5
	const { data: plans, error } = await supabase.rpc('plans');
	if (error) throw error;
	return { data, plans };
}
