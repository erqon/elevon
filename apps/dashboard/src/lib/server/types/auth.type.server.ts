export interface LoginResponse {
  sessionToken: string;
}

export interface User {
  id: string;
  email: string;
  firstName: string | null;
  lastName: string;
  createdAt: Date;
  sessionId: string;
}
