import { usePromise } from '@/hooks/use-promise';
import UserService from '@/grpc-services/user';
import PeopleService from "@/grpc-services/people";

const userService  = new UserService();
const peopleService = new PeopleService();

export default function useService() {
  return {
    user: {
      getMe: usePromise(userService.getMe.bind(userService), []),
    },
    people: {
      find: usePromise(peopleService.findUser.bind(peopleService), ['userOrderId'])
    }
  }
}
