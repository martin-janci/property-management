import "@typespec/http";
import "@typespec/openapi";
import "@typespec/openapi3";

// Import shared models
import "./shared/models.tsp";
import "./shared/errors.tsp";
import "./shared/pagination.tsp";
import "./shared/tenant.tsp";

// Import domain specifications
import "./domains/auth.tsp";
import "./domains/organizations.tsp";
import "./domains/buildings.tsp";
import "./domains/units.tsp";
import "./domains/faults.tsp";
import "./domains/voting.tsp";
import "./domains/announcements.tsp";
import "./domains/documents.tsp";
import "./domains/rentals.tsp";
import "./domains/listings.tsp";
import "./domains/compliance.tsp";
import "./domains/accounting.tsp";

using TypeSpec.Http;
using TypeSpec.OpenAPI;

@service(#{
  title: "Property Management API",
})
@info(#{
})
@server("https://api.property-management.example.com", "Production")
@server("http://localhost:8080", "Development")
namespace PropertyManagement;
