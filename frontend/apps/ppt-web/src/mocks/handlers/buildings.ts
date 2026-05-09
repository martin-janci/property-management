import { HttpResponse, http } from 'msw';
import { seedBuildings } from '../seeds/data';

export const buildingsHandlers = [
  http.get('*/api/buildings', () => HttpResponse.json(seedBuildings)),
  http.get('*/api/buildings/:id', ({ params }) => {
    const b = seedBuildings.find((x) => x.id === params.id);
    return b ? HttpResponse.json(b) : new HttpResponse(null, { status: 404 });
  }),
];
