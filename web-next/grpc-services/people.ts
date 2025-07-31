import {
    PeopleService as PeopleClient,
    Client,
    FindUserRequestSchema,
} from 'schema-ts'
import {create} from "@bufbuild/protobuf";
export default class PeopleService extends Client.Base<typeof PeopleClient> {
    constructor() {
        super(PeopleClient);
    }

    public async findUser(userOrderId: number) {
        const request = create(FindUserRequestSchema, {
            orderId: BigInt(userOrderId)
        });

        const response = await this.client.find_user(request);
        return response.user
    }
}
