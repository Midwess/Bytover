import {ReceiveSessionViewModel, ImageReceiveResourceViewModel, AvatarViewModel, SelectedResourceViewModel, LocalResourcePathVariantAbsolutePath, ResourceTypeVariantImage} from 'shared_types/types/shared_types'

// Create image resources using test data images
export const receiveImageResources: ImageReceiveResourceViewModel[] = [
    new ImageReceiveResourceViewModel(
        new SelectedResourceViewModel(
            1n,
            "image1.jpg",
            0.0,
            1.6,
            "/test_data/image1.jpg",
            new LocalResourcePathVariantAbsolutePath("/test_data/image1.jpg"),
            null,
            new ResourceTypeVariantImage()
        ),
        false
    ),
    new ImageReceiveResourceViewModel(
        new SelectedResourceViewModel(
            2n,
            "image2.jpg", 
            0.0,
            4.0,
            "/test_data/image2.jpg",
            new LocalResourcePathVariantAbsolutePath("/test_data/image2.jpg"),
            null,
            new ResourceTypeVariantImage()
        ),
        true
    ),
    new ImageReceiveResourceViewModel(
        new SelectedResourceViewModel(
            3n,
            "image3.jpg",
            0.0,
            4.4,
            "/test_data/image3.jpg", 
            new LocalResourcePathVariantAbsolutePath("/test_data/image3.jpg"),
            null,
            new ResourceTypeVariantImage()
        ),
        false
    ),
    new ImageReceiveResourceViewModel(
        new SelectedResourceViewModel(
            4n,
            "image4.jpg",
            0.0,
            0.44,
            "/test_data/image4.jpg",
            new LocalResourcePathVariantAbsolutePath("/test_data/image4.jpg"),
            null,
            new ResourceTypeVariantImage()
        ),
        true
    ),
    new ImageReceiveResourceViewModel(
        new SelectedResourceViewModel(
            5n,
            "image5.jpg",
            0.0,
            3.4,
            "/test_data/image5.jpg",
            new LocalResourcePathVariantAbsolutePath("/test_data/image5.jpg"),
            null,
            new ResourceTypeVariantImage()
        ),
        false
    ),
    new ImageReceiveResourceViewModel(
        new SelectedResourceViewModel(
            6n,
            "image6.jpg",
            0.0,
            8.5,
            "/test_data/image6.jpg",
            new LocalResourcePathVariantAbsolutePath("/test_data/image6.jpg"),
            null,
            new ResourceTypeVariantImage()
        ),
        true
    )
]

export const receiveList: ReceiveSessionViewModel[] = [
        new ReceiveSessionViewModel(
            1n,
            new AvatarViewModel(
                "https://via.placeholder.com/150",
                null,
                null,
                null
            ),
            "John Doe",
            "192.168.1.100",
            receiveImageResources,
            [],
            [],
            true,
            false,
            "1MB/s",
            1,
            "2024-01-15 10:35",
        ),
        new ReceiveSessionViewModel(
            2n,
            new AvatarViewModel(
                "https://via.placeholder.com/150",
                null,
                null,
                null
            ),
            "Jane Smith",
            "192.168.1.101",
            receiveImageResources,
            [],
            [],
            true,
            false,
            "2MB/s",
            1,
            "2024-01-15 09:15",
        ),
        new ReceiveSessionViewModel(
            3n,
            new AvatarViewModel(
                "https://via.placeholder.com/150",
                null,
                null,
                null
            ),
            "Mike Johnson",
            "192.168.1.102",
            receiveImageResources,
            [],
            [],
            false,
            true,
            "1.5MB/s",
            0.6,
            "2024-01-15 11:00",
        ),
        new ReceiveSessionViewModel(
            4n,
            new AvatarViewModel(
                "https://via.placeholder.com/150",
                null,
                null,
                null
            ),
            "Sarah Wilson",
            "192.168.1.103",
            receiveImageResources,
            [],
            [],
            false,
            false,
            "0MB/s",
            0,
            "2024-01-15 08:45",
        ),
        new ReceiveSessionViewModel(
            5n,
            new AvatarViewModel(
                "https://via.placeholder.com/150",
                null,
                null,
                null
            ),
            "David Brown",
            "192.168.1.104",
            receiveImageResources,
            [],
            [],
            false,
            true,
            "800KB/s",
            0.25,
            "2024-01-15 12:20",
        ),
        new ReceiveSessionViewModel(
            6n,
            new AvatarViewModel(
                "https://via.placeholder.com/150",
                null,
                null,
                null
            ),
            "Emily Davis",
            "192.168.1.105",
            receiveImageResources,
            [],
            [],
            true,
            false,
            "1.2MB/s",
            1,
            "2024-01-15 07:30",
        ),
        new ReceiveSessionViewModel(
            7n,
            new AvatarViewModel(
                "https://via.placeholder.com/150",
                null,
                null,
                null
            ),
            "Alex Miller",
            "192.168.1.106",
            receiveImageResources,
            [],
            [],
            false,
            false,
            "0MB/s",
            0,
            "2024-01-15 13:10",
        ),
        new ReceiveSessionViewModel(
            8n,
            new AvatarViewModel(
                "https://via.placeholder.com/150",
                null,
                null,
                null
            ),
            "Lisa Garcia",
            "192.168.1.107",
            receiveImageResources,
            [],
            [],
            false,
            true,
            "1.1MB/s",
            0.3,
            "2024-01-15 14:00",
        ),
        new ReceiveSessionViewModel(
            9n,
            new AvatarViewModel(
                "https://via.placeholder.com/150",
                null,
                null,
                null
            ),
            "Tom Anderson",
            "192.168.1.108",
            receiveImageResources,
            [],
            [],
            true,
            false,
            "900KB/s",
            1,
            "2024-01-15 06:15",
        ),
        new ReceiveSessionViewModel(
            10n,
            new AvatarViewModel(
                "https://via.placeholder.com/150",
                null,
                null,
                null
            ),
            "Rachel White",
            "192.168.1.109",
            receiveImageResources,
            [],
            [],
            false,
            false,
            "0MB/s",
            0,
            "2024-01-15 15:45",
        )
    ]