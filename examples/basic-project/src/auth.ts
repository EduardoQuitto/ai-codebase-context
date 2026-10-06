export interface User {
  id: string;
  email: string;
}

export class AuthService {
  verify(token: string): boolean {
    return token.length > 0;
  }
}
