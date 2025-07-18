import {
    MeRequestSchema,
    UserService as UserClient,
    Client,
    User
} from 'schema-ts';
import { create } from '@bufbuild/protobuf';

export default class UserService extends Client.Base<typeof UserClient> {
    constructor() {
        super(UserClient);
    }

    public async getMe(): Promise<User | null> {
        console.log('tiendang-debug', 'getMe')
        const accessToken = localStorage.getItem('access_token')
        if (!accessToken) return null

        const request = create(MeRequestSchema, {
            conditions: []
        });

        const response = await this.client.me(request);
        return response.user || null
    }
}
