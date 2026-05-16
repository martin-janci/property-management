import { Route, Routes } from 'react-router-dom';

export function App() {
  return (
    <Routes>
      <Route path="*" element={<div>PPT Admin (scaffolding)</div>} />
    </Routes>
  );
}
