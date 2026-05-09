import { HttpResponse, http } from 'msw';
import { seedListings } from '../seeds/data';

export const listingsHandlers = [http.get('*/api/listings', () => HttpResponse.json(seedListings))];
